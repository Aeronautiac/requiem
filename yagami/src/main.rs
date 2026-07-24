mod constants;

use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Write,
    net::SocketAddr,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path, Query, State, WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::IntoResponse,
    routing::{any, post},
};
use lawliet_types::action::ActionRequest;
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::mpsc::{self, Receiver},
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use crate::constants::{OUTBOX_BUF_SIZE, TICKET_LIMIT, TICKET_TIMEOUT};

fn req(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("missing required env var: {key}"))
}

struct Config {
    bind_addr: SocketAddr,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        Ok(Config {
            bind_addr: req("YAGAMI_BIND")?
                .parse()
                .map_err(|e| format!("YAGAMI_BIND: {e}"))?,
        })
    }
}

enum ServerOutput {
    Ping,
}

// controls handled a level above the engine by the game task (undo N, evict key, reboot) -- they act
// ON the engine/timeline, not IN the fiction. reboot has no live engine to reach at all.
enum GameControl {}

enum ServerInput {
    Pong,
    Action(ActionRequest),
    Control(GameControl),
}

// carries the source ticket so the game task can route replies and enforce permissions.
struct InputEnvelope {
    ticket: Ticket,
    input: ServerInput,
}

type Token = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
struct Key(Token);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
struct Ticket(Token);

impl Key {
    fn generate() -> Self {
        Self(generate_token())
    }
}

impl Ticket {
    fn generate() -> Self {
        Self(generate_token())
    }
}

impl IntoResponse for Ticket {
    fn into_response(self) -> axum::response::Response {
        self.0.into_response() // text/plain, same as when Ticket was a bare String
    }
}

type GameId = u64; // for now, strictly incrementing

struct KeyData {
    cancel: CancellationToken,
    tickets: HashSet<Ticket>,
}

struct ConnHandle {
    cancel: CancellationToken,
    outbox: mpsc::Sender<ServerOutput>,
    // set when the game task cuts this connection; the connection task hasn't torn down yet. fan-out
    // skips a dropped entry in the window between the cancel and the ClaimGuard actually removing it.
    dropped: bool,
}

struct GameHandle {
    inbox: mpsc::Sender<ServerInput>,
    tickets: HashMap<Ticket, Key>,
    connections: HashMap<Ticket, ConnHandle>,
    keys: HashMap<Key, KeyData>,
}

#[derive(Default)]
struct ServerState {
    games: HashMap<GameId, GameHandle>,
}
type WrappedServerState = Arc<Mutex<ServerState>>;

// a poisoned lock means a thread panicked mid-mutation, so the maps can no longer be trusted. that
// is a process-wide problem: take the process down and let the supervisor restart us.
//
// deliberately not unwrap(): a panic here would be caught at the tokio task boundary, killing one
// task while leaving the poisoned state in place and the server running. abort is the only response
// that is loud and deterministic regardless of whether we are already unwinding.
fn lock_state(state: &Mutex<ServerState>) -> MutexGuard<'_, ServerState> {
    state.lock().unwrap_or_else(|_| {
        eprintln!("server state mutex poisoned -- aborting");
        std::process::abort()
    })
}

fn generate_token() -> Token {
    let mut bytes = [0u8; 32]; // 256 bits of entropy
    getrandom::fill(&mut bytes).expect("OS CSPRNG unavailable");

    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let config = Config::from_env().expect("config");

    let server_state = Arc::new(Mutex::new(ServerState::default()));

    let router = Router::new()
        .route("/game/{id}/get_ticket", post(get_ticket))
        .route("/game/{id}/ws", any(establish_ws_connection))
        .with_state(server_state.clone());

    let listener = TcpListener::bind(config.bind_addr).await.unwrap();

    axum::serve(listener, router).await.unwrap();
}

#[derive(Deserialize)]
struct TicketRequest {
    key: Key,
}

#[derive(Serialize)]
enum ServerError {
    InvalidGameId,
    InvalidKey,
    InvalidTicket,
    TicketLimitReached,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::InvalidGameId => StatusCode::NOT_FOUND,
            Self::InvalidKey => StatusCode::NOT_FOUND,
            Self::TicketLimitReached => StatusCode::FORBIDDEN,
            Self::InvalidTicket => StatusCode::NOT_FOUND,
        };
        (status, Json(self)).into_response()
    }
}

async fn get_ticket(
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    Json(body): Json<TicketRequest>,
) -> Result<Ticket, ServerError> {
    let mut server_state = lock_state(&state);

    let Some(game_state) = server_state.games.get_mut(&game_id) else {
        return Err(ServerError::InvalidGameId);
    };

    let key = body.key;
    let Some(key_data) = game_state.keys.get_mut(&key) else {
        return Err(ServerError::InvalidKey);
    };

    if key_data.tickets.len() == TICKET_LIMIT {
        return Err(ServerError::TicketLimitReached);
    }

    let ticket = Ticket::generate();
    key_data.tickets.insert(ticket.clone());
    game_state.tickets.insert(ticket.clone(), key.clone());

    let state_clone = state.clone();
    let ticket_clone = ticket.clone();
    tokio::spawn(async move {
        sleep(Duration::from_secs(TICKET_TIMEOUT)).await;
        let mut server_state = lock_state(&state_clone);
        if let Some(game_state) = server_state.games.get_mut(&game_id)
            && !game_state.connections.contains_key(&ticket_clone)
        {
            game_state.tickets.remove(&ticket_clone);
            if let Some(key_data) = game_state.keys.get_mut(&key) {
                key_data.tickets.remove(&ticket_clone);
            }
        }
    });

    Ok(ticket)
}

#[derive(Deserialize)]
struct JoinGame {
    ticket: Ticket,
}

// releases a claim made by establish_ws_connection.
//
// held by the on_upgrade callback rather than being cleanup at the end of game_connection, because
// axum drops the callback *uncalled* when the upgrade fails (the Err arm of `on_upgrade.await` in
// axum's ws.rs). hyper writes the 101 from its own connection task after our handler has already
// returned, so a claim made here can outlive an upgrade that never completes. dropping a closure
// drops its captures, so this runs on that path too -- as well as on panic and on cancellation.
struct ClaimGuard {
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
            && let Some(key_data) = game_state.keys.get_mut(&key)
        {
            key_data.tickets.remove(&self.ticket);
        }
    }
}

// websocket upgrade and ticket claim (dont put the claim in the connection handler. it creates a
// race window.)
//
// claiming here is what makes the 101 authoritative: a client holding one holds a claim, so every
// post-101 failure is a transport failure rather than an authorization one. the client's rule stays
// "4xx means don't retry, dead socket means retry" with no ambiguous state in between.
async fn establish_ws_connection(
    ws: WebSocketUpgrade,
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    Query(params): Query<JoinGame>,
) -> Result<axum::response::Response, ServerError> {
    let mut server_state = lock_state(&state);

    let Some(game_state) = server_state.games.get_mut(&game_id) else {
        return Err(ServerError::InvalidGameId);
    };
    let Some(key) = game_state.tickets.get(&params.ticket).cloned() else {
        return Err(ServerError::InvalidTicket);
    };

    // tickets are single use. presence in `connections` is the claimed test -- the ledger keeps
    // ticket -> key for the life of the connection, so `tickets` alone cannot answer this.
    // reported as InvalidTicket so a replay cannot distinguish "claimed" from "never existed".
    if game_state.connections.contains_key(&params.ticket) {
        return Err(ServerError::InvalidTicket);
    }

    let Some(KeyData { cancel, .. }) = game_state.keys.get(&key) else {
        // the server is broken if this happens
        // a key removal should remove all tickets associated with that key as well
        unreachable!();
    };
    let cancel = cancel.child_token();

    let (outbox, inbox) = mpsc::channel(OUTBOX_BUF_SIZE);

    game_state.connections.insert(
        params.ticket.clone(),
        ConnHandle {
            cancel,
            outbox,
            dropped: false,
        },
    );
    drop(server_state);

    // only construct the guard once the claim has actually been made. building it before the
    // rejection checks above would mean a rejected replay drops a guard on the way out and reaps
    // the ticket belonging to the live connection it collided with.
    let guard = ClaimGuard {
        state: state.clone(),
        game_id,
        ticket: params.ticket,
    };

    Ok(ws.on_upgrade(move |socket| async move {
        let _guard = guard; // released when the connection ends, or if the upgrade never completes
        game_connection(socket, state, inbox).await;
    }))
}

// handles a game connection lifecycle
async fn game_connection(
    stream: WebSocket,
    state: WrappedServerState,
    recv: mpsc::Receiver<ServerOutput>,
) -> impl IntoResponse {
}
