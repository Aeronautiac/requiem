// The axum surface: configuration, the REST endpoints, and the websocket upgrade plus its pump.
//
// Everything here is edge work -- parsing, authenticating, and handing off. Once a socket is live
// its traffic belongs to the game task, and this module only shuttles bytes between the two.

use std::{
    convert::Infallible, env, fmt::Display, future::ready, net::SocketAddr, num::NonZeroU32,
    str::FromStr, time::Duration,
};

use axum::{
    Json,
    extract::{
        FromRequestParts, OptionalFromRequestParts, Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header, request::Parts},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use tokio::{
    select,
    sync::mpsc::{self},
    time::{interval, sleep, timeout},
};
use tokio_util::sync::CancellationToken;

use yagami_wire::{PrivilegeSet, generate_token};

use crate::{
    account::{Account, AccountId, Role, hash_password, session_token_hash, verify_password},
    auth::{ActorScope, Capability, Key, KeyHandle, Ticket},
    delivery::DeliveryData,
    game::{GameCommand, GameInput, GameStart, InputEnvelope, game, minted_key, wake},
    state::{ConnHandle, GameHandle, GameId, Settings, WrappedServerState, lock_state},
    store::{SignupRejected, Store},
    wire::{
        AdminControl, Batch, ServerInput, SimControl, SimControlData,
    },
};

pub fn req(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("missing required env var: {key}"))
}

fn req_parse<T: FromStr>(key: &str) -> Result<T, String>
where
    T::Err: Display,
{
    req(key)?.parse().map_err(|e| format!("{key}: {e}"))
}

// one token every `period_key` milliseconds, saving up to `burst_key` while idle.
fn req_quota(period_key: &str, burst_key: &str) -> Result<Quota, String> {
    let period = req_millis(period_key)?;
    let quota = Quota::with_period(period).ok_or(format!("{period_key}: must be nonzero"))?;
    Ok(quota.allow_burst(req_parse::<NonZeroU32>(burst_key)?))
}

fn req_secs(key: &str) -> Result<Duration, String> {
    Ok(Duration::from_secs(req_parse(key)?))
}

fn req_millis(key: &str) -> Result<Duration, String> {
    Ok(Duration::from_millis(req_parse(key)?))
}

pub struct Config {
    pub bind_addr: SocketAddr,
    pub allowed_origin: HeaderValue,
    pub database_url: String,
    pub database_pool_size: u32,
    pub settings: Settings,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Config {
            bind_addr: req_parse("YAGAMI_BIND")?,
            allowed_origin: req_parse("YAGAMI_ALLOWED_ORIGIN")?,
            database_url: req("DATABASE_URL")?,
            database_pool_size: req_parse("YAGAMI_DATABASE_POOL_SIZE")?,
            settings: Settings {
                max_resident: req_parse("YAGAMI_MAX_RESIDENT")?,
                hibernate_after: req_secs("YAGAMI_HIBERNATE_AFTER_SECS")?,
                null_tick_interval: req_secs("YAGAMI_NULL_TICK_INTERVAL_SECS")?,
                engine_timeout: req_secs("YAGAMI_ENGINE_TIMEOUT_SECS")?,
                boot_retry_base: req_millis("YAGAMI_BOOT_RETRY_BASE_MS")?,
                boot_max_retries: req_parse("YAGAMI_BOOT_MAX_RETRIES")?,
                boot_cooldown: req_secs("YAGAMI_BOOT_COOLDOWN_SECS")?,
                ticket_limit: req_parse("YAGAMI_TICKET_LIMIT")?,
                ticket_timeout: req_secs("YAGAMI_TICKET_TIMEOUT_SECS")?,
                outbox_size: req_parse("YAGAMI_OUTBOX_SIZE")?,
                batch_size: req_parse("YAGAMI_BATCH_SIZE")?,
                heartbeat_interval: req_secs("YAGAMI_HEARTBEAT_INTERVAL_SECS")?,
                heartbeat_timeout: req_secs("YAGAMI_HEARTBEAT_TIMEOUT_SECS")?,
                max_input_bytes: req_parse("YAGAMI_MAX_INPUT_BYTES")?,
                max_body_bytes: req_parse("YAGAMI_MAX_BODY_BYTES")?,
                input_quota: req_quota("YAGAMI_INPUT_PERIOD_MS", "YAGAMI_INPUT_BURST")?,
                request_quota: req_quota("YAGAMI_REQUEST_PERIOD_MS", "YAGAMI_REQUEST_BURST")?,
                auth_quota: req_quota("YAGAMI_AUTH_PERIOD_MS", "YAGAMI_AUTH_BURST")?,
                session_ttl: req_secs("YAGAMI_SESSION_TTL_SECS")?,
                password_min_len: req_parse("YAGAMI_PASSWORD_MIN_LEN")?,
                password_max_len: req_parse("YAGAMI_PASSWORD_MAX_LEN")?,
                account_game_quota: req_parse("YAGAMI_ACCOUNT_GAME_QUOTA")?,
                create_requires_verified: req_parse("YAGAMI_CREATE_REQUIRES_VERIFIED")?,
            },
        })
    }
}

#[derive(Serialize)]
pub enum ServerError {
    InvalidGameId,
    InvalidKey,
    InvalidTicket,
    TicketLimitReached,
    // the game could not be brought up: a new game failed its first boot, or an existing one gave
    // up booting recently and is refused a wake until its cooldown passes.
    GameBootFailed,
    // as many games are running as the server allows; nothing new may start until one hibernates.
    ServerAtCapacity,
    // the database could not answer.
    StoreFailed,
    // this client IP is over its REST budget.
    RateLimited,
    // no live session came with the request.
    NotLoggedIn,
    // no account has this username and password. which of the two was wrong is not said.
    InvalidCredentials,
    UsernameTaken,
    InvalidUsername,
    // outside the allowed password length.
    InvalidPassword,
    // creating games currently requires a verified account.
    VerificationRequired,
    // the account already owns as many active games as it may.
    GameQuotaReached,
    // the account already has a game being created.
    CreationInProgress,
    // only the game's owner or an admin may do this.
    NotGameOwner,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::InvalidGameId => StatusCode::NOT_FOUND,
            Self::InvalidKey => StatusCode::NOT_FOUND,
            Self::TicketLimitReached => StatusCode::FORBIDDEN,
            Self::InvalidTicket => StatusCode::NOT_FOUND,
            Self::GameBootFailed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ServerAtCapacity => StatusCode::SERVICE_UNAVAILABLE,
            Self::StoreFailed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::NotLoggedIn => StatusCode::UNAUTHORIZED,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::UsernameTaken => StatusCode::CONFLICT,
            Self::InvalidUsername => StatusCode::BAD_REQUEST,
            Self::InvalidPassword => StatusCode::BAD_REQUEST,
            Self::VerificationRequired => StatusCode::FORBIDDEN,
            Self::GameQuotaReached => StatusCode::FORBIDDEN,
            Self::CreationInProgress => StatusCode::CONFLICT,
            Self::NotGameOwner => StatusCode::FORBIDDEN,
        };
        (status, Json(self)).into_response()
    }
}

// ===== ACCOUNTS ===== //

// the __Host- prefix makes the browser refuse the cookie unless it is Secure, host-only and Path=/,
// so no other subdomain can set or shadow it.
const SESSION_COOKIE: &str = "__Host-session";

// the cookie only carries the token. the session's real lifetime is its row's expiry, pushed out on
// every use, so the cookie asks for the longest life browsers grant (they cap it at 400 days).
const SESSION_COOKIE_MAX_AGE_SECS: u64 = 400 * 24 * 60 * 60;

fn session_cookie(token: &str, max_age_secs: u64) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Max-Age={max_age_secs}; Path=/; Secure; HttpOnly; SameSite=Lax"
    ))
    .expect("a hex token is a valid header value")
}

fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| pair.trim().strip_prefix(SESSION_COOKIE)?.strip_prefix('='))
}

// the session cookie resolved to its account, with the session's expiry pushed out. None without a
// live session.
async fn session_account(
    headers: &HeaderMap,
    state: &WrappedServerState,
) -> Result<Option<Account>, ServerError> {
    let Some(token) = session_token(headers) else {
        return Ok(None);
    };
    let (store, ttl) = {
        let server_state = lock_state(state);
        (server_state.store.clone(), server_state.settings.session_ttl)
    };
    store
        .session_account(&session_token_hash(token), ttl)
        .await
        .map_err(|e| {
            eprintln!("failed to resolve a session: {e}");
            ServerError::StoreFailed
        })
}

// an endpoint that requires a login: rejected NotLoggedIn without a live session.
impl FromRequestParts<WrappedServerState> for Account {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &WrappedServerState,
    ) -> Result<Self, ServerError> {
        session_account(&parts.headers, state)
            .await?
            .ok_or(ServerError::NotLoggedIn)
    }
}

// an endpoint that only benefits from a login. it never fails because of one: a missing, stale or
// unresolvable session is just None.
impl OptionalFromRequestParts<WrappedServerState> for Account {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &WrappedServerState,
    ) -> Result<Option<Self>, Infallible> {
        Ok(session_account(&parts.headers, state).await.unwrap_or(None))
    }
}

#[derive(Deserialize)]
pub struct Credentials {
    username: String,
    password: String,
}

type SessionCookie = [(HeaderName, HeaderValue); 1];

// log an account in: mint a session and hand its token to the browser as the session cookie.
async fn start_session(
    store: &Store,
    account_id: AccountId,
    ttl: Duration,
) -> Result<SessionCookie, ServerError> {
    let token = generate_token();
    if let Err(e) = store
        .create_session(&session_token_hash(&token), account_id, ttl)
        .await
    {
        eprintln!("failed to create a session for account {account_id}: {e}");
        return Err(ServerError::StoreFailed);
    }
    Ok([(
        header::SET_COOKIE,
        session_cookie(&token, SESSION_COOKIE_MAX_AGE_SECS),
    )])
}

fn check_password_length(password: &str, settings: Settings) -> Result<(), ServerError> {
    let length = password.chars().count();
    if length < settings.password_min_len || length > settings.password_max_len {
        return Err(ServerError::InvalidPassword);
    }
    Ok(())
}

// create an account and log it in. the username rules live in the accounts table's CHECK.
pub async fn signup(
    State(state): State<WrappedServerState>,
    Json(body): Json<Credentials>,
) -> Result<SessionCookie, ServerError> {
    let (store, settings) = {
        let server_state = lock_state(&state);
        (server_state.store.clone(), server_state.settings)
    };
    check_password_length(&body.password, settings)?;
    let password_hash = hash_password(body.password).await;
    let account_id = match store.create_account(&body.username, &password_hash).await {
        Ok(Ok(account_id)) => account_id,
        Ok(Err(SignupRejected::UsernameTaken)) => return Err(ServerError::UsernameTaken),
        Ok(Err(SignupRejected::InvalidUsername)) => return Err(ServerError::InvalidUsername),
        Err(e) => {
            eprintln!("failed to create an account: {e}");
            return Err(ServerError::StoreFailed);
        }
    };
    start_session(&store, account_id, settings.session_ttl).await
}

pub async fn login(
    State(state): State<WrappedServerState>,
    Json(body): Json<Credentials>,
) -> Result<SessionCookie, ServerError> {
    let (store, settings) = {
        let server_state = lock_state(&state);
        (server_state.store.clone(), server_state.settings)
    };
    let (account_id, password_hash) = match store.load_credentials(&body.username).await {
        Ok(Some(credentials)) => credentials,
        Ok(None) => return Err(ServerError::InvalidCredentials),
        Err(e) => {
            eprintln!("failed to load credentials: {e}");
            return Err(ServerError::StoreFailed);
        }
    };
    if !verify_password(body.password, password_hash).await {
        return Err(ServerError::InvalidCredentials);
    }
    start_session(&store, account_id, settings.session_ttl).await
}

// end this session and clear its cookie. succeeds without a session, so a stale client can always
// get back to a clean state.
pub async fn logout(
    State(state): State<WrappedServerState>,
    headers: HeaderMap,
) -> Result<SessionCookie, ServerError> {
    if let Some(token) = session_token(&headers) {
        let store = lock_state(&state).store.clone();
        if let Err(e) = store.delete_session(&session_token_hash(token)).await {
            eprintln!("failed to delete a session: {e}");
            return Err(ServerError::StoreFailed);
        }
    }
    Ok([(header::SET_COOKIE, session_cookie("", 0))])
}

#[derive(Deserialize)]
pub struct PasswordChange {
    current: String,
    new: String,
}

// change the password, then end every other session: a password is usually changed because someone
// else might know the old one. this session stays logged in.
pub async fn change_password(
    State(state): State<WrappedServerState>,
    headers: HeaderMap,
    account: Account,
    Json(body): Json<PasswordChange>,
) -> Result<(), ServerError> {
    let (store, settings) = {
        let server_state = lock_state(&state);
        (server_state.store.clone(), server_state.settings)
    };
    check_password_length(&body.new, settings)?;
    let current_hash = match store.load_credentials(&account.username).await {
        Ok(Some((_, current_hash))) => current_hash,
        Ok(None) => return Err(ServerError::NotLoggedIn), // deleted since the session resolved
        Err(e) => {
            eprintln!("failed to load credentials: {e}");
            return Err(ServerError::StoreFailed);
        }
    };
    if !verify_password(body.current, current_hash).await {
        return Err(ServerError::InvalidCredentials);
    }
    let token = session_token(&headers).ok_or(ServerError::NotLoggedIn)?;
    let new_hash = hash_password(body.new).await;
    if let Err(e) = store
        .change_password(account.id, &new_hash, &session_token_hash(token))
        .await
    {
        eprintln!("failed to change account {}'s password: {e}", account.id);
        return Err(ServerError::StoreFailed);
    }
    Ok(())
}

#[derive(Serialize)]
pub struct AccountPacket {
    #[serde(flatten)]
    account: Account,
    // active games it owns.
    games: Vec<GameId>,
    // keys it has joined with, for the quick-join menus, each with what it permits so a menu can say
    // what joining with it gets you.
    keys: Vec<AccountKey>,
}

// which saved key: how a client names one to forget it.
#[derive(Serialize, Deserialize)]
pub struct SavedKey {
    game_id: GameId,
    key: Key,
}

#[derive(Serialize)]
pub struct AccountKey {
    #[serde(flatten)]
    saved: SavedKey,
    privileges: PrivilegeSet,
}

// the account this session is logged in as, with its games and saved keys. how a client learns, on
// load, whether it is logged in; refetched after anything that changes it.
pub async fn account(
    State(state): State<WrappedServerState>,
    account: Account,
) -> Result<Json<AccountPacket>, ServerError> {
    let store = lock_state(&state).store.clone();
    let id = account.id;
    let failed = |e: sqlx::Error| {
        eprintln!("failed to load account {id}'s games and keys: {e}");
        ServerError::StoreFailed
    };
    let games = store.owned_games(id).await.map_err(failed)?;
    let keys = store
        .saved_keys(id)
        .await
        .map_err(failed)?
        .into_iter()
        .map(|(game_id, key, privileges)| AccountKey {
            saved: SavedKey { game_id, key },
            privileges,
        })
        .collect();
    Ok(Json(AccountPacket {
        account,
        games,
        keys,
    }))
}

// remove a saved key from the account's menus. the key itself is untouched.
pub async fn forget_key(
    State(state): State<WrappedServerState>,
    account: Account,
    Json(body): Json<SavedKey>,
) -> Result<(), ServerError> {
    let store = lock_state(&state).store.clone();
    store
        .forget_key(account.id, body.game_id, &body.key)
        .await
        .map_err(|e| {
            eprintln!("failed to forget a key for account {}: {e}", account.id);
            ServerError::StoreFailed
        })
}

// ===== TICKETS ===== //

#[derive(Deserialize)]
pub struct TicketRequest {
    key: Key,
}

// joining needs only a key. a logged-in caller also gets the key saved to their account, for the
// quick-join menus; the ticket is theirs either way, so a failed save is only logged.
pub async fn get_ticket(
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    account: Option<Account>,
    Json(body): Json<TicketRequest>,
) -> Result<Ticket, ServerError> {
    let ticket = ticket_for_key(&state, game_id, body.key.clone()).await?;
    if let Some(account) = account {
        let store = lock_state(&state).store.clone();
        if let Err(e) = store.save_key(account.id, game_id, &body.key).await {
            eprintln!("failed to save a key for account {}: {e}", account.id);
        }
    }
    Ok(ticket)
}

// a running game answers from its handle. a hibernated one has no handle: its key is checked
// against the durable ledger first -- so no key can wake a game it does not belong to -- and then
// the game is woken and the ticket issued against its fresh handle. a game caught mid-hibernation is
// waited out and then woken, so the caller never sees the difference.
async fn ticket_for_key(
    state: &WrappedServerState,
    game_id: GameId,
    key: Key,
) -> Result<Ticket, ServerError> {
    loop {
        // subscribed under the lock, while the handle still exists, so its removal cannot be missed.
        let closing = {
            let mut server_state = lock_state(state);
            let settings = server_state.settings;
            match server_state.games.get_mut(&game_id) {
                Some(game_state) if game_state.closing => Some(game_state.gone.subscribe()),
                Some(game_state) => {
                    return issue_ticket(state, settings, game_state, game_id, key);
                }
                None => None,
            }
        };
        // changed() only errors once the handle is dropped (nothing is ever sent), after which the
        // game is plainly asleep -- or ended.
        if let Some(mut gone) = closing {
            let _ = gone.changed().await;
            continue;
        }

        let store = lock_state(state).store.clone();
        let keys = match store.load_keys(game_id).await {
            Ok(Some(keys)) => keys,
            Ok(None) => return Err(ServerError::InvalidGameId),
            Err(e) => {
                eprintln!("failed to load keys for game {game_id}: {e}");
                return Err(ServerError::StoreFailed);
            }
        };
        if !keys.contains_key(&key) {
            return Err(ServerError::InvalidKey);
        }

        let mut server_state = lock_state(state);
        // another request may have woken it while the ledger was loading: go again against its
        // handle.
        if server_state.games.contains_key(&game_id) {
            continue;
        }
        if server_state.boot_cooling_down(game_id) {
            return Err(ServerError::GameBootFailed);
        }
        if server_state.at_capacity() {
            return Err(ServerError::ServerAtCapacity);
        }
        wake(state, &mut server_state, game_id, keys);
        let settings = server_state.settings;
        let game_state = server_state
            .games
            .get_mut(&game_id)
            .expect("just woken, under this lock");
        return issue_ticket(state, settings, game_state, game_id, key);
    }
}

fn issue_ticket(
    state: &WrappedServerState,
    settings: Settings,
    game_state: &mut GameHandle,
    game_id: GameId,
    key: Key,
) -> Result<Ticket, ServerError> {
    if !game_state.keys.contains_key(&key) {
        return Err(ServerError::InvalidKey);
    }
    let Some(key_handle) = game_state.key_handles.get_mut(&key) else {
        return Err(ServerError::InvalidKey);
    };

    if key_handle.tickets.len() >= settings.ticket_limit {
        return Err(ServerError::TicketLimitReached);
    }

    let ticket = Ticket::generate();
    key_handle.tickets.insert(ticket.clone());
    game_state.tickets.insert(ticket.clone(), key.clone());

    let state_clone = state.clone();
    let ticket_clone = ticket.clone();
    tokio::spawn(async move {
        sleep(settings.ticket_timeout).await;
        let mut server_state = lock_state(&state_clone);
        if let Some(game_state) = server_state.games.get_mut(&game_id)
            && !game_state.connections.contains_key(&ticket_clone)
        {
            game_state.tickets.remove(&ticket_clone);
            if let Some(key_handle) = game_state.key_handles.get_mut(&key) {
                key_handle.tickets.remove(&ticket_clone);
            }
        }
    });

    Ok(ticket)
}

#[derive(Deserialize)]
pub struct JoinGame {
    ticket: Ticket,
}

// releases a claim made by establish_ws_connection.
//
// held by the on_upgrade callback rather than being cleanup at the end of game_connection, because
// axum drops the callback *uncalled* when the upgrade fails (the Err arm of `on_upgrade.await` in
// axum's ws.rs). hyper writes the 101 from its own connection task after our handler has already
// returned, so a claim made here can outlive an upgrade that never completes. dropping a closure
// drops its captures, so this runs on that path too -- as well as on panic and on cancellation.
pub struct ClaimGuard {
    state: WrappedServerState,
    game_id: GameId,
    ticket: Ticket,
}

impl Drop for ClaimGuard {
    fn drop(&mut self) {
        // no .await in here -- Drop is synchronous. that is why this is a std Mutex.
        let mut server_state = lock_state(&self.state);

        let Some(game_state) = server_state.games.get_mut(&self.game_id) else {
            return; // game is gone, its maps went with it
        };

        game_state.connections.remove(&self.ticket);

        // single use: the ticket dies with the connection it was claimed for
        if let Some(key) = game_state.tickets.remove(&self.ticket)
            && let Some(key_handle) = game_state.key_handles.get_mut(&key)
        {
            key_handle.tickets.remove(&self.ticket);
        }
    }
}

// websocket upgrade and ticket claim (dont put the claim in the connection handler. it creates a
// race window.)
//
// claiming here is what makes the 101 authoritative: a client holding one holds a claim, so every
// post-101 failure is a transport failure rather than an authorization one. the client's rule stays
// "4xx means don't retry, dead socket means retry" with no ambiguous state in between.
pub async fn establish_ws_connection(
    ws: WebSocketUpgrade,
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    Query(params): Query<JoinGame>,
) -> Result<axum::response::Response, ServerError> {
    let mut server_state = lock_state(&state);

    let settings = server_state.settings;
    let ws = ws
        .max_message_size(settings.max_input_bytes)
        .max_frame_size(settings.max_input_bytes);

    let Some(game_state) = server_state.games.get_mut(&game_id) else {
        return Err(ServerError::InvalidGameId);
    };
    // a closing game is semantically gone: its tickets died with it, even though the handle has not
    // left the registry yet. the client's retry goes back through get_ticket, which waits it out.
    if game_state.closing {
        return Err(ServerError::InvalidTicket);
    }
    let Some(key) = game_state.tickets.get(&params.ticket).cloned() else {
        return Err(ServerError::InvalidTicket);
    };

    // tickets are single use. presence in `connections` is the claimed test -- the ledger keeps
    // ticket -> key for the life of the connection, so `tickets` alone cannot answer this.
    // reported as InvalidTicket so a replay cannot distinguish "claimed" from "never existed".
    if game_state.connections.contains_key(&params.ticket) {
        return Err(ServerError::InvalidTicket);
    }

    let Some(KeyHandle { cancel, .. }) = game_state.key_handles.get(&key) else {
        // the server is broken if this happens
        // a key removal should remove all tickets associated with that key as well
        unreachable!();
    };
    let cancel = cancel.child_token();

    let (outbox, inbox) = mpsc::channel(settings.outbox_size);

    game_state.connections.insert(
        params.ticket.clone(),
        ConnHandle {
            cancel,
            outbox,
            dropped: false,
            // delivery data starts empty; the game task fills it as it replays/syncs this socket.
            delivery: DeliveryData::default(),
        },
    );
    drop(server_state);

    // only construct the guard once the claim has actually been made. building it before the
    // rejection checks above would mean a rejected replay drops a guard on the way out and reaps
    // the ticket belonging to the live connection it collided with.
    let guard = ClaimGuard {
        state: state.clone(),
        game_id,
        ticket: params.ticket.clone(),
    };

    Ok(ws.on_upgrade(move |socket| async move {
        let _guard = guard; // released when the connection ends, or if the upgrade never completes
        game_connection(socket, state, inbox, game_id, params.ticket).await;
    }))
}

pub async fn game_connection(
    stream: WebSocket,
    state: WrappedServerState,
    mut recv: mpsc::Receiver<Batch>,
    game_id: GameId,
    ticket: Ticket,
) {
    let (mut ws_send, mut ws_recv) = stream.split();
    let (cancel_tok, inbox, settings) = {
        let server_state = lock_state(&state);
        let Some(game_state) = server_state.games.get(&game_id) else {
            return;
        };
        let Some(conn_handle) = game_state.connections.get(&ticket) else {
            // invalid state
            std::process::abort();
        };
        (
            conn_handle.cancel.clone(),
            game_state.inbox.clone(),
            server_state.settings,
        )
    };
    let input_budget = RateLimiter::direct(settings.input_quota);

    if inbox
        .send(GameInput::GameCommand(GameCommand::Sync {
            ticket: ticket.clone(),
        }))
        .is_err()
    {
        return; // game task is gone
    }

    let mut inbound = tokio::spawn(async move {
        loop {
            // per-iteration timeout is the heartbeat deadline: every inbound frame grants the next
            // read a fresh window, so silence past the heartbeat timeout is what marks a dead peer.
            let msg = match timeout(settings.heartbeat_timeout, ws_recv.next()).await {
                Err(_) => break,                  // no frame within the deadline -> dead peer
                Ok(None | Some(Err(_))) => break, // stream ended / transport error
                Ok(Some(Ok(msg))) => msg,
            };

            match msg {
                Message::Text(t) => {
                    let Ok(input) = serde_json::from_str::<ServerInput>(t.as_str()) else {
                        break; // undeserializable payload -> protocol violation
                    };

                    // over budget, stop reading until a token frees up. the backlog waits in the
                    // socket's fixed-size kernel buffer and then on the sender's side, never here.
                    // the wait is outside the read timeout, so it cannot trip the heartbeat.
                    input_budget.until_ready().await;

                    if inbox
                        .send(GameInput::ServerInput(InputEnvelope {
                            ticket: ticket.clone(),
                            input,
                        }))
                        .is_err()
                    {
                        break; // game task is gone
                    }
                }
                Message::Ping(_) | Message::Pong(_) => {} // liveness only; deadline already reset
                Message::Binary(_) => break,              // protocol violation
                Message::Close(_) => break,
            }
        }
    });

    let mut outbound = tokio::spawn(async move {
        let mut split_batches: Option<std::vec::IntoIter<Batch>> = None;
        let mut ping = interval(settings.heartbeat_interval);
        loop {
            select! {
                // always send a ping on first opportunity
                biased;

                _ = ping.tick() => {
                    if ws_send.send(Message::Ping(Default::default())).await.is_err() {
                        break; // socket gone
                    }
                },

                // we have an unsplit batch waiting. only take it when we have already processed the
                // previous batch. split it into chunks.
                out = recv.recv(), if split_batches.is_none() => match out {
                    Some(out) => {
                        // split batch into chunks using batch split method (chunks are just more batches)
                        split_batches = Some(out.into_chunks(settings.batch_size).into_iter());
                    }
                    None => break, // game task dropped the outbox sender
                },

                () = ready(()), if split_batches.is_some() => {
                    let Some(batches) = &mut split_batches  else {
                        unreachable!()
                    };
                    let out = batches.next();

                    // Batch is ours; a serialize failure is a bug in this process, not a
                    // runtime condition. abort loudly rather than drop a message on the floor.
                    let json = serde_json::to_string(&out).unwrap_or_else(|e| {
                        eprintln!("Batch failed to serialize: {e} -- aborting");
                        std::process::abort()
                    });
                    if ws_send.send(Message::Text(json.into())).await.is_err() {
                        break; // socket gone
                    }

                    if batches.len() == 0 {
                        split_batches = None;
                    }
                }
            }
        }
    });

    select! {
        _ = cancel_tok.cancelled() => {}
        _ = &mut inbound => {}
        _ = &mut outbound => {}
    }
    inbound.abort();
    outbound.abort();
}

#[derive(Serialize)]
pub struct RosterEntry {
    game_id: GameId,
    // Sockets currently open to the game. Always 0 for a hibernated game.
    connections: usize,
    // Keys held in the game -- the people. A key is an identity (a set of privileges), and it
    // counts as a person whether or not it is currently connected; multiple tickets (connections)
    // can map to the same key.
    keys: usize,
    // Whether the game is running right now, rather than hibernated.
    resident: bool,
}

// The platform's directory of active games, running or hibernated. Unauthenticated: a game id and
// its headcounts are public presence info, not a secret. Polled by the platform screen on an
// interval.
pub async fn roster(
    State(state): State<WrappedServerState>,
) -> Result<Json<Vec<RosterEntry>>, ServerError> {
    let store = lock_state(&state).store.clone();
    let listings = store.game_directory().await.map_err(|e| {
        eprintln!("failed to read the game directory: {e}");
        ServerError::StoreFailed
    })?;
    let server_state = lock_state(&state);
    // the directory is ordered by id, which keeps the client's re-render from reshuffling rows
    // between polls.
    let entries = listings
        .into_iter()
        .map(|listing| {
            let handle = server_state.games.get(&listing.game_id);
            RosterEntry {
                game_id: listing.game_id,
                connections: handle.map_or(0, |handle| handle.connections.len()),
                keys: listing.keys,
                resident: handle.is_some(),
            }
        })
        .collect();
    Ok(Json(entries))
}

#[derive(Serialize)]
pub struct GameCreationPacket {
    game_id: GameId,
    admin_key: Key,
}

// a fresh game's hold on a running slot (and on its creator's one creation in flight), from
// create_game's checks until the game task takes it over. dropped before hand_off -- the handler
// bailed, or was cancelled mid-await by its client going away -- it gives both back.
struct CreationReservation {
    state: WrappedServerState,
    creator: Option<AccountId>,
    held: bool,
}

impl CreationReservation {
    fn take(
        state: &WrappedServerState,
        creator: Option<AccountId>,
    ) -> Result<Self, ServerError> {
        let mut server_state = lock_state(state);
        if let Some(creator) = creator
            && server_state.creators.contains(&creator)
        {
            return Err(ServerError::CreationInProgress);
        }
        if server_state.at_capacity() {
            return Err(ServerError::ServerAtCapacity);
        }
        server_state.reserve_creation(creator);
        Ok(Self {
            state: state.clone(),
            creator,
            held: true,
        })
    }

    // the game task releases it from here on: on registering, or in its Drop.
    fn hand_off(mut self) -> Option<AccountId> {
        self.held = false;
        self.creator
    }
}

impl Drop for CreationReservation {
    fn drop(&mut self) {
        if self.held {
            lock_state(&self.state).release_creation(self.creator);
        }
    }
}

// create a game owned by the caller. returns its id and its admin key, which is also saved to the
// account. a game is only written to the DB once it boots, so a failure here leaves nothing behind.
pub async fn create_game(
    State(state): State<WrappedServerState>,
    account: Account,
) -> Result<Json<GameCreationPacket>, ServerError> {
    let (store, settings) = {
        let server_state = lock_state(&state);
        (server_state.store.clone(), server_state.settings)
    };
    let admin = account.role == Role::Admin;
    if settings.create_requires_verified && !account.verified && !admin {
        return Err(ServerError::VerificationRequired);
    }

    // reserve first, count second: while the reservation is held nothing else can create for this
    // account, and the task writes the owner row before releasing it, so the count can't go stale.
    let reservation = CreationReservation::take(&state, Some(account.id))?;
    if !admin {
        match store.owned_games(account.id).await {
            Ok(owned) if owned.len() >= settings.account_game_quota => {
                return Err(ServerError::GameQuotaReached);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to count account {}'s games: {e}", account.id);
                return Err(ServerError::StoreFailed);
            }
        }
    }

    let creation_pack = vec![ServerInput::Control(AdminControl::Sim(SimControl {
        time: 0,
        data: SimControlData::CreateKey {
            actors: ActorScope::All,
            capabilities: vec![Capability::Administer, Capability::Supervise],
        },
    }))];

    let (inbox, events) = mpsc::unbounded_channel();
    let cancel = CancellationToken::new();
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    // the task boots, writes itself to the DB, and replies with the game id + the creation pack's
    // responses (incl. the minted admin key). a game that fails to boot replies Err and is never
    // written, so nothing to clean up here.
    tokio::spawn(game(
        state.clone(),
        GameStart::Fresh {
            creator: reservation.hand_off(),
            creation_pack,
            creation_reply: reply_tx,
        },
        events,
        inbox,
        cancel,
    ));

    let (game_id, responses) = match reply_rx.await {
        Ok(Ok(ok)) => ok,
        _ => return Err(ServerError::GameBootFailed),
    };

    let Some(admin_key) = minted_key(&responses) else {
        return Err(ServerError::GameBootFailed);
    };
    Ok(Json(GameCreationPacket { game_id, admin_key }))
}

// the one way a game ends, for its owner or an admin.
//
// this only ASKS. it does not remove the registry entry, close sockets or reap the child -- the game
// task owns all of that and does it once, on its way out, no matter which path started it. so a
// teardown that races another teardown, or a game that is already dying, needs no special handling.
pub async fn end_game(
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    account: Account,
) -> Result<(), ServerError> {
    let store = lock_state(&state).store.clone();
    let owner = match store.game_owner(game_id).await {
        Ok(Some(owner)) => owner,
        Ok(None) => return Err(ServerError::InvalidGameId),
        Err(e) => {
            eprintln!("failed to load game {game_id}'s owner: {e}");
            return Err(ServerError::StoreFailed);
        }
    };
    if account.role != Role::Admin && owner != Some(account.id) {
        return Err(ServerError::NotGameOwner);
    }

    // mark it ended first, so nothing can wake it again -- a wake already in flight finds no active
    // row to load and exits.
    match store.end_game(game_id).await {
        Ok(true) => {}
        Ok(false) => return Err(ServerError::InvalidGameId),
        Err(e) => {
            eprintln!("failed to mark game {game_id} ended: {e}");
            return Err(ServerError::StoreFailed);
        }
    }

    // a hibernated game has no task to stop.
    if let Some(game) = lock_state(&state).games.get_mut(&game_id) {
        game.close();
    }

    Ok(())
}
