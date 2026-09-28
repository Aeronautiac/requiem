// yagami: the central server. hosts many lawliet engines, one child process per game, and routes
// their command streams to connected clients under a per-key privilege set.
//
// The split, roughly outermost-inwards:
//   wire      -- what goes over the socket. the shapes amane's hand-written bindings.ts mirrors.
//   http      -- the axum surface: game creation, tickets, the websocket upgrade and its pump.
//   auth      -- keys, tickets, capabilities, actor scope. what a credential is allowed to be.
//   control   -- key management and the Supervise invariant, which must change as one unit.
//   state     -- the in-memory registry of games and their live connections.
//   game      -- one game's coordinator task: the engine child, its log, and fan-out.
//   delivery  -- who receives which command, in what order. the server-side access control.

mod account;
mod auth;
mod delivery;
mod game;
mod http;
mod state;
mod store;
mod wire;

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::DefaultBodyLimit,
    http::{Method, header},
    response::IntoResponse,
    routing::{any, delete, get, post, put},
};
use lawliet_types::common::Seed;
use tokio::net::TcpListener;
use governor::{Quota, middleware::NoOpMiddleware};
use tower_governor::{
    GovernorError, GovernorLayer, governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
};
use tower_http::cors::CorsLayer;

use crate::{
    http::{
        Config, ServerError, account, change_password, create_game, end_game,
        establish_ws_connection, forget_key, get_ticket, login, logout, roster, signup,
    },
    state::ServerState,
    store::Store,
};

// Server-wide primitives, kept at the root because they belong to no one module: the game task
// stamps actions with `now`, and game creation seeds an engine with `generate_seed`.
pub fn generate_seed() -> Seed {
    let mut bytes = [0u8; size_of::<Seed>()];
    getrandom::fill(&mut bytes).expect("OS CSPRNG unavailable");
    Seed::from_le_bytes(bytes)
}

// a per-client-IP limit, read from X-Forwarded-For. Caddy overwrites that header with the real peer,
// so it is only trustworthy while Caddy is the sole way in: yagami must not be reachable directly.
// with no header (local dev, no proxy) it falls back to the socket's peer address.
fn ip_limit(quota: Quota) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body> {
    let config = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .period(quota.replenish_interval())
        .burst_size(quota.burst_size().get())
        .finish()
        .expect("a Quota is never zero");
    // the limiter keeps a bucket per IP it has seen; forget the ones that have refilled.
    let buckets = config.limiter().clone();
    tokio::spawn(async move {
        let mut sweep = tokio::time::interval(Duration::from_secs(60));
        loop {
            sweep.tick().await;
            buckets.retain_recent();
        }
    });
    GovernorLayer::new(config).error_handler(|error| match error {
        GovernorError::TooManyRequests { .. } => ServerError::RateLimited.into_response(),
        error => error.into(),
    })
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let config = Config::from_env().expect("config");

    let store = Store::connect(&config.database_url, config.database_pool_size)
        .await
        .expect("failed to connect to postgres");

    // nothing is resumed here: after a restart every active game is simply hibernated, and wakes
    // on the first get_ticket for it like any other sleeping game.
    let settings = config.settings;
    let server_state = Arc::new(Mutex::new(ServerState {
        store,
        games: HashMap::new(),
        creating: 0,
        creators: HashSet::new(),
        boot_failures: HashMap::new(),
        settings,
    }));

    // REST is cross-origin (client on a different subdomain), so the JSON POST triggers a preflight
    // the browser blocks on until we answer. the WS route is exempt -- same-origin policy doesn't
    // cover websockets -- so this layer is only about the fetch-based endpoints. it is the outermost
    // layer, so preflights are answered before the rate limit and a RateLimited answer is still
    // readable by the browser. credentials let the session cookie ride along; the browser only
    // allows that with an explicit origin, never a wildcard.
    let cors = CorsLayer::new()
        .allow_origin(config.allowed_origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    // the websocket route spends a ticket per connection, so get_ticket's limit already covers it.
    let rest = Router::new()
        .route("/create_game", post(create_game))
        .route("/roster", get(roster))
        .route("/game/{id}/end_game", post(end_game))
        .route("/game/{id}/get_ticket", post(get_ticket))
        .route("/auth/logout", post(logout))
        .route("/account", get(account))
        .route("/account/keys", delete(forget_key))
        .layer(ip_limit(settings.request_quota));

    // everything that checks a password.
    let auth = Router::new()
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/account/password", put(change_password))
        .layer(ip_limit(settings.auth_quota));

    let router = Router::new()
        .merge(rest)
        .merge(auth)
        .route("/game/{id}/ws", any(establish_ws_connection))
        .layer(DefaultBodyLimit::max(settings.max_body_bytes))
        .layer(cors)
        .with_state(server_state.clone());

    let listener = TcpListener::bind(config.bind_addr).await.unwrap();

    // connect info gives the rate limiter the peer address to fall back on.
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
