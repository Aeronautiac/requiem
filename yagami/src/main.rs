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

mod constants;

mod auth;
mod delivery;
mod game;
mod http;
mod state;
mod wire;

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    http::{Method, header},
    routing::{any, post},
};
use lawliet_types::common::Seed;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::{
    http::{Config, create_game, end_game, establish_ws_connection, get_ticket},
    state::ServerState,
};

// Server-wide primitives, kept at the root because they belong to no one module: the game task
// stamps actions with `now`, and game creation seeds an engine with `generate_seed`.
pub fn generate_seed() -> Seed {
    let mut bytes = [0u8; size_of::<Seed>()];
    getrandom::fill(&mut bytes).expect("OS CSPRNG unavailable");
    Seed::from_le_bytes(bytes)
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let config = Config::from_env().expect("config");

    let server_state = Arc::new(Mutex::new(ServerState::default()));

    // REST is cross-origin (client on a different subdomain), so the JSON POST triggers a preflight
    // the browser blocks on until we answer. the WS route is exempt -- same-origin policy doesn't
    // cover websockets -- so this layer is only about the fetch-based endpoints.
    let cors = CorsLayer::new()
        .allow_origin(config.allowed_origin)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let router = Router::new()
        .route("/create_game", post(create_game))
        .route("/game/{id}/end_game", post(end_game))
        .route("/game/{id}/get_ticket", post(get_ticket))
        .route("/game/{id}/ws", any(establish_ws_connection))
        .layer(cors)
        .with_state(server_state.clone());

    let listener = TcpListener::bind(config.bind_addr).await.unwrap();

    axum::serve(listener, router).await.unwrap();
}
