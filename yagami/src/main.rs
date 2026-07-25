mod constants;

use std::{
    collections::{HashMap, HashSet},
    env::{self, current_exe},
    fmt::Write,
    net::SocketAddr,
    process::Stdio,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderValue, Method, StatusCode, header},
    response::IntoResponse,
    routing::{any, post},
};
use enumflags2::{BitFlags, bitflags};
use futures_util::{SinkExt, StreamExt};
use lawliet_types::{
    action::{Action, ActionActor, ActionError, ActionRequest, ActionResponse, InitializeEngine},
    command::{CommandPayload, CommandRecipient},
    common::{ActorKey, Seed, Time},
    engine::ExecutionResult,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::TcpListener,
    process::{ChildStdin, ChildStdout},
    select,
    sync::mpsc::{self},
    time::{Instant, interval, sleep, sleep_until, timeout},
};
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

use crate::constants::{
    ENGINE_TIMEOUT, HEARTBEAT_INTERVAL, HEARTBEAT_TIMEOUT, OUTBOX_BUF_SIZE, TICKET_LIMIT,
    TICKET_TIMEOUT,
};

fn req(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("missing required env var: {key}"))
}

struct Config {
    bind_addr: SocketAddr,
    allowed_origin: HeaderValue,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        Ok(Config {
            bind_addr: req("YAGAMI_BIND")?
                .parse()
                .map_err(|e| format!("YAGAMI_BIND: {e}"))?,
            allowed_origin: req("YAGAMI_ALLOWED_ORIGIN")?
                .parse()
                .map_err(|e| format!("YAGAMI_ALLOWED_ORIGIN: {e}"))?,
        })
    }
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

// what a control produced. the game-administration counterpart to ActionResponse.
#[derive(Serialize)]
enum ControlResponse {
    Ended,
    KeyCreated { key: Key },
    KeyRevoked,
    CapabilitiesSet,
    ActorScopeSet,
}

// a control refused on its own terms -- the caller IS an administrator, but not for this particular
// target. distinct from Denied, which means they are not an administrator at all.
#[derive(Serialize)]
enum ControlError {
    KeyNotFound,
    // the caller holds Supervise and aimed at its own key
    CannotActOnSelf,
    // the target holds Administer and the caller does not hold Supervise
    RequiresSupervise,
    // Supervise may only come from someone who already holds it
    CannotGrantSupervise,
}

// split by what was asked, then by how it went, so the outcome of an action can never be read as the
// outcome of a control and each side owns its own error type.
#[derive(Serialize)]
enum ExecOutcome {
    Action(ActionOutcome),
    Control(ControlOutcome),
}

// the ActionContext is NOT inlined here -- its commands ride the enclosing Batch's command buffer, so
// a reply and a push deliver state by the exact same path and the client has one place to apply from.
#[derive(Serialize)]
enum ActionOutcome {
    Ok(ActionResponse),
    Err(ActionError),
    // this key may not act as the requested actor. decided here, never sent to the engine, which has
    // no concept of connections or keys.
    //
    // answered rather than punished: anyone can write a client, and a UI offering something the key
    // cannot do is a bad UI, not an attack. cutting the socket over it would be.
    Denied,
    // the engine child died with this action in flight. the action is the prime suspect, so it is
    // NOT logged and NOT replayed into the fresh child.
    Crashed,
}

#[derive(Serialize)]
enum ControlOutcome {
    Ok(ControlResponse),
    Err(ControlError),
    // same meaning as its ActionOutcome twin: this key does not permit what was asked. here it means
    // the key holds no administration capability at all.
    Denied,
}

// the reply echoes the input it answers, so it covers actions and controls alike -- and the client
// can match a reply to what it sent without the server inventing a correlation id.
#[derive(Serialize)]
struct ResponsePair {
    input: ServerInput,
    output: ExecOutcome,
}

// commands are already recipient-filtered for the connection this is addressed to. `response` is set
// only on the connection that submitted the action.
#[derive(Serialize)]
struct Batch {
    commands: Vec<CommandPayload>,
    response: Option<ResponsePair>,
}

#[derive(Serialize)]
enum OutputData {
    Batch(Batch),
}

#[derive(Serialize)]
struct ServerOutput {
    seq_num: u64,
    data: OutputData,
}

// controls handled a level above the engine by the game task (undo N, evict key, reboot) -- they act
// ON the engine/timeline, not IN the fiction. reboot has no live engine to reach at all.
// Serialize as well as Deserialize because a reply echoes the input it answers (see ResponsePair).
// every variant here needs Administer. beyond that, authority over the TARGET key is decided by
// may_manage -- see Capability::Supervise.
//
// both mutators REPLACE rather than delta, so the admin's client always states the complete intended
// privilege set and there is no read-modify-write to get wrong.
#[derive(Serialize, Deserialize)]
enum GameControl {
    // tear this game down: engine child, connections, registry entry. the game admin's route to the
    // same teardown a platform admin reaches over REST.
    EndGame,
    // mint a key for this game. this is how a player is let in: create a key scoped to their
    // actor(s), then hand it over out of band.
    CreateKey {
        actors: ActorScope,
        capabilities: Vec<Capability>,
    },
    RevokeKey {
        key: Key,
    },
    SetCapabilities {
        key: Key,
        capabilities: Vec<Capability>,
    },
    SetActorScope {
        key: Key,
        actors: ActorScope,
    },
}

#[derive(Serialize, Deserialize)]
enum ServerInput {
    Action(ActionRequest),
    Control(GameControl),
}

// carries the source ticket so the game task can route replies and enforce permissions.
struct InputEnvelope {
    ticket: Ticket,
    input: ServerInput,
}

// the action the engine is currently working on.
struct InFlight {
    // who is waiting on it. `None` for actions the server issues on its own behalf -- engine
    // initialization -- which have no originating connection to reply to. their commands are still
    // logged and fanned out like any other; only the reply has nowhere to go.
    ticket: Option<Ticket>,
    request: ActionRequest,
}

// everything the game task hears about. one channel, not two, so ordering is free: a connection's
// Attach is queued before any input it goes on to send, so it is always replayed before it can act.
enum GameEvent {
    // a websocket finished upgrading and wants its catch-up replay.
    Attach { ticket: Ticket },
    Input(InputEnvelope),
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

// a capability is a thing a key may do that isn't "act as an actor". there is deliberately NO
// admin-vs-player key TYPE -- "admin" is just the key whose privilege set is maximal. the game only
// ever asks "does this set permit X". adding capabilities is additive.
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Capability {
    // act as ActionActor::Admin, observe the System command stream, and manage keys.
    Administer = 1 << 0,
    // authority over OTHER administrators' keys. without it an admin can manage ordinary keys but
    // cannot touch a key that holds Administer -- which is what stops someone handed admin from
    // turning around and revoking it from the host who handed it to them.
    //
    // it can only ever come from someone who already holds it, and its holder cannot edit its own
    // key: authority over admins sits ABOVE admins, including above the holder's own reach.
    Supervise = 1 << 1,
}

// the wire carries a list of names rather than a bitmask so a hand-written client never has to know
// bit values; BitFlags is the in-memory representation only.
fn to_flags(capabilities: &[Capability]) -> BitFlags<Capability> {
    capabilities
        .iter()
        .fold(BitFlags::empty(), |flags, capability| flags | *capability)
}

// which actors a key may act as / observe. `All` is not the same as a set holding every actor that
// exists today: it covers actors created LATER, so an admin key needs no bookkeeping when the engine
// adds a player. an `Only` set is enumerated at mint time and never has to be topped up.
#[derive(Clone, Serialize, Deserialize)]
enum ActorScope {
    All,
    Only(HashSet<ActorKey>),
}

impl ActorScope {
    fn contains(&self, actor: &ActorKey) -> bool {
        match self {
            Self::All => true,
            Self::Only(actors) => actors.contains(actor),
        }
    }
}

// what a key is allowed to do. resolved from a ticket at the moment of use and never copied into the
// connection -- so narrowing or revoking a key takes effect on its live sockets immediately.
struct Privileges {
    actors: ActorScope,
    capabilities: BitFlags<Capability>,
}

impl Privileges {
    // may a connection holding this set submit an action as this actor?
    fn can_act_as(&self, actor: &ActionActor) -> bool {
        match actor {
            ActionActor::Admin => self.capabilities.contains(Capability::Administer),
            // NO key may act as System, not even an admin one. System is the server's own voice and
            // it reaches machinery that exists precisely to be out of participants' hands -- an
            // admin holding it could tear down state (channels, prosecutions) that is not supposed
            // to be tearable. yagami's own System actions never arrive on a connection, so they
            // never pass through here.
            ActionActor::System => false,
            ActionActor::Player(id) => self.actors.contains(id),
            // an org is never acted AS from a connection. a player who wants an org to do something
            // sends their own player-level action, and the engine instantiates the org action from
            // it -- so the org's authority stays the engine's to grant, never a client's to claim.
            ActionActor::Organization(_) => false,
        }
    }
}

struct KeyData {
    cancel: CancellationToken,
    tickets: HashSet<Ticket>,
    privileges: Privileges,
}

struct ConnHandle {
    cancel: CancellationToken,
    outbox: mpsc::Sender<ServerOutput>,
    // set when the game task cuts this connection; the connection task hasn't torn down yet. fan-out
    // skips a dropped entry in the window between the cancel and the ClaimGuard actually removing it.
    dropped: bool,
    // this connection's own sequence counter. per-connection and runtime-only: it counts batches
    // THIS socket was sent, so it is dense with no gaps, which is what lets the client treat a gap as
    // a desync. 0 means nothing sent yet; the first batch is 1.
    seq_num: u64,
}

struct GameHandle {
    // root of this game's token tree: every key token is a child of it and every connection token a
    // child of one of those, so cancelling here reaches all of them without walking the maps.
    cancel: CancellationToken,
    inbox: mpsc::UnboundedSender<GameEvent>,
    tickets: HashMap<Ticket, Key>,
    connections: HashMap<Ticket, ConnHandle>,
    keys: HashMap<Key, KeyData>,
}

impl GameHandle {
    // ticket -> key -> privilege set. the ledger holds ticket->key for the life of the connection, so
    // this resolves for as long as the connection is claimed.
    fn privileges(&self, ticket: &Ticket) -> Option<&Privileges> {
        let key = self.tickets.get(ticket)?;
        Some(&self.keys.get(key)?.privileges)
    }
}

// may a connection holding `privileges` receive the command sitting at log position `pos`? this is
// the whole of the server-side access control on state -- everything the client does with visibility
// beyond this is UX, not security.
//
// takes the position and `born` because BasePlayer is not a flat "everyone" stream: it is what an
// actor needs in order to have arrived late, so a base command belongs to actors NEWER than it. an
// actor that already existed when it was emitted learned the same thing through its own stream and
// must not be handed it a second time on reconnect.
fn can_see(
    privileges: &Privileges,
    born: &HashMap<ActorKey, usize>,
    pos: usize,
    recipient: &CommandRecipient,
) -> bool {
    match recipient {
        CommandRecipient::System => privileges.capabilities.contains(Capability::Administer),
        CommandRecipient::BasePlayer => match &privileges.actors {
            ActorScope::All => true,
            ActorScope::Only(actors) => actors
                .iter()
                .any(|actor| born.get(actor).is_some_and(|&birth| birth > pos)),
        },
        CommandRecipient::Actor(id) => privileges.actors.contains(id),
    }
}

#[derive(Default)]
struct ServerState {
    next_game_id: GameId,
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

fn generate_seed() -> Seed {
    let mut bytes = [0u8; size_of::<Seed>()];
    getrandom::fill(&mut bytes).expect("OS CSPRNG unavailable");
    Seed::from_le_bytes(bytes)
}

fn now() -> Time {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the unix epoch")
        .as_millis()
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

#[derive(Deserialize)]
struct TicketRequest {
    key: Key,
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
            seq_num: 0,
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

async fn game_connection(
    stream: WebSocket,
    state: WrappedServerState,
    mut recv: mpsc::Receiver<ServerOutput>,
    game_id: GameId,
    ticket: Ticket,
) {
    let (mut ws_send, mut ws_recv) = stream.split();
    let (cancel_tok, inbox) = {
        let server_state = lock_state(&state);
        let Some(game_state) = server_state.games.get(&game_id) else {
            return;
        };
        let Some(conn_handle) = game_state.connections.get(&ticket) else {
            // invalid state
            std::process::abort();
        };
        (conn_handle.cancel.clone(), game_state.inbox.clone())
    };

    // ask for the catch-up replay before reading a single frame. it goes down the same channel as
    // inputs, so FIFO guarantees this connection is caught up before anything it sends is executed --
    // no window in which a reply could be ordered ahead of the log it depends on.
    if inbox
        .send(GameEvent::Attach {
            ticket: ticket.clone(),
        })
        .is_err()
    {
        return; // game task is gone
    }

    let mut inbound = tokio::spawn(async move {
        loop {
            // per-iteration timeout is the heartbeat deadline: every inbound frame grants the next
            // read a fresh window, so silence past HEARTBEAT_TIMEOUT is what marks a dead peer.
            let msg = match timeout(Duration::from_secs(HEARTBEAT_TIMEOUT), ws_recv.next()).await {
                Err(_) => break,                  // no frame within the deadline -> dead peer
                Ok(None | Some(Err(_))) => break, // stream ended / transport error
                Ok(Some(Ok(msg))) => msg,
            };

            match msg {
                Message::Text(t) => {
                    let Ok(mut input) = serde_json::from_str::<ServerInput>(t.as_str()) else {
                        break; // undeserializable payload -> protocol violation
                    };

                    // game time is the SERVER's, so whatever the client put here is overwritten
                    // rather than trusted or even treated as a hint -- a client that lies, or whose
                    // clock is simply wrong, cannot move the engine's clock or backdate an action.
                    //
                    // stamped on arrival, here, and not in the coordinator: an action that queues
                    // behind another would otherwise be recorded at the time it got SERVICED, which
                    // is the queue's delay rather than the player's.
                    if let ServerInput::Action(request) = &mut input {
                        request.timestamp = now();
                    }
                    if inbox
                        .send(GameEvent::Input(InputEnvelope {
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
        let mut ping = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        loop {
            select! {
                // provoke a Pong so the peer's recv-side deadline keeps resetting on an idle link.
                _ = ping.tick() => {
                    if ws_send.send(Message::Ping(Default::default())).await.is_err() {
                        break; // socket gone
                    }
                }
                out = recv.recv() => match out {
                    Some(out) => {
                        // ServerOutput is ours; a serialize failure is a bug in this process, not a
                        // runtime condition. abort loudly rather than drop a message on the floor.
                        let json = serde_json::to_string(&out).unwrap_or_else(|e| {
                            eprintln!("ServerOutput failed to serialize: {e} -- aborting");
                            std::process::abort()
                        });
                        if ws_send.send(Message::Text(json.into())).await.is_err() {
                            break; // socket gone
                        }
                    }
                    None => break, // game task dropped the outbox sender
                }
            }
        }
    });

    // &mut so the handles survive the race; whichever arm wins, abort the other two tasks so the
    // socket halves drop and the connection actually closes. abort on a finished task is a no-op.
    select! {
        _ = cancel_tok.cancelled() => {}
        _ = &mut inbound => {}
        _ = &mut outbound => {}
    }
    inbound.abort();
    outbound.abort();
}

#[derive(Deserialize)]
struct CreateGame {
    platform_key: String, // these are strings because they are created explicitly by a platform admin
}

#[derive(Serialize)]
struct GameCreationPacket {
    game_id: GameId,
    admin_key: Key,
}

// who may create and tear down games at all. server-wide, and one level ABOVE per-game keys: a game
// knows only its own keys, never this. kept a seam so swapping the placeholder for real accounts
// never touches the key model.
fn is_platform_admin(_platform_key: &str) -> bool {
    // TODO: back this with a gitignored allowlist file. open during testing.
    true
}

// need a REST endpoint for game creation
// to create a game, you must have a platform key
// returns the id of the game created and the admin key
// must create the game entry in the REST endpoint, but cleanup can be handled outside of it (games
// are guaranteed to be created after auth, so there is no failure case with a weird cleanup scenario).
//
// the engine child boots with the game task, but an empty engine is NOT a playable game: the admin
// still has to send InitializeEngine and InitializeWorld as its first actions. creation deliberately
// drives none of that -- it mints one credential and gets out of the way, so nothing here has to
// await the engine.
async fn create_game(
    State(state): State<WrappedServerState>,
    Json(body): Json<CreateGame>,
) -> Result<Json<GameCreationPacket>, ServerError> {
    if !is_platform_admin(&body.platform_key) {
        return Err(ServerError::InvalidKey);
    }

    let admin_key = Key::generate();
    let (inbox, events) = mpsc::unbounded_channel();
    let cancel = CancellationToken::new();

    let game_id = {
        let mut server_state = lock_state(&state);

        let game_id = server_state.next_game_id;
        server_state.next_game_id += 1;

        server_state.games.insert(
            game_id,
            GameHandle {
                cancel: cancel.clone(),
                inbox,
                tickets: HashMap::new(),
                connections: HashMap::new(),
                keys: HashMap::from([(
                    admin_key.clone(),
                    // child of the game token, so tearing the game down takes this key's
                    // connections with it.
                    KeyData {
                        cancel: cancel.child_token(),
                        tickets: HashSet::new(),
                        // the maximal set, and the only place Supervise originates -- the host holds
                        // authority over any admin key it later hands out. `All` rather than an
                        // enumerated set so players created later need no re-grant.
                        privileges: Privileges {
                            actors: ActorScope::All,
                            capabilities: Capability::Administer | Capability::Supervise,
                        },
                    },
                )]),
            },
        );

        game_id
    };

    // registered before the task is spawned, so a get_ticket that races the spawn still finds the
    // game -- its events just queue on the unbounded inbox until the task starts draining.
    tokio::spawn(game(state, game_id, events, cancel));

    Ok(Json(GameCreationPacket { game_id, admin_key }))
}

// serialize something of ours for the wire. a failure is a bug in this process, not a runtime
// condition, so abort loudly rather than paper over a half-written protocol.
fn to_line<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(json) => json + "\n",
        Err(e) => {
            eprintln!("failed to serialize for the engine pipe: {e} -- aborting");
            std::process::abort()
        }
    }
}

// hand one batch to one connection, stamped with that connection's next sequence number.
//
// best effort by design: a client whose outbox is full is CUT, not waited on. the alternatives are
// unbounded memory growth or letting it silently miss mandatory state, and a client missing state is
// worse than a client that has to reconnect.
fn push_batch(conn: &mut ConnHandle, batch: Batch) {
    conn.seq_num += 1;
    let output = ServerOutput {
        seq_num: conn.seq_num,
        data: OutputData::Batch(batch),
    };

    if conn.outbox.try_send(output).is_err() {
        conn.cancel.cancel();
        conn.dropped = true;
    }
}

// may the caller act on this target key? the single authority rule for every key-management control,
// kept in one place so a control added later cannot quietly skip it.
//
// the two rules below combine into the property that keeps a game administrable: a key holding
// Administer is reachable ONLY from a Supervise holder, and a Supervise holder cannot reach its own
// key -- so the LAST Supervise holder can be neither revoked nor demoted, by anyone. there is always
// at least one key holding Administer. changing either rule breaks that, so change them together.
//
// note this is not a count-the-admins guard: a lone PLAIN admin may still revoke itself and cut
// itself off. that is deliberate. the case being prevented is nobody holding admin, and the
// unreachable supervisor is what prevents it.
fn may_manage(
    game: &GameHandle,
    caller_key: &Key,
    supervises: bool,
    target: &Key,
) -> Result<(), ControlError> {
    let Some(target_data) = game.keys.get(target) else {
        return Err(ControlError::KeyNotFound);
    };

    if target == caller_key {
        // a plain admin has full authority over itself, up to and including revoking its own key and
        // cutting itself off. a supervisor deliberately does not: authority over admins sits above
        // admins, and that has to include the holder's own key or it is just a self-granted crown.
        return if supervises {
            Err(ControlError::CannotActOnSelf)
        } else {
            Ok(())
        };
    }

    // another administrator's key is reachable only from above.
    if target_data
        .privileges
        .capabilities
        .contains(Capability::Administer)
        && !supervises
    {
        return Err(ControlError::RequiresSupervise);
    }

    Ok(())
}

// withdraw a key and everything standing on it.
fn revoke_key(game: &mut GameHandle, key: &Key) {
    let Some(key_data) = game.keys.remove(key) else {
        return;
    };

    // ends every socket opened with this key at once -- their tokens are children of this one.
    key_data.cancel.cancel();

    for ticket in key_data.tickets {
        // the ledger entry has to go with the key: an unclaimed ticket that resolves to a key which
        // no longer exists is a live panic path in establish_ws_connection.
        game.tickets.remove(&ticket);

        // a claimed ticket still has its ConnHandle until the connection task's guard runs. mark it
        // so fan-out skips it in that window: it can no longer be resolved to a privilege set, and
        // both dispatch and attach treat an unresolvable LIVE connection as a broken invariant.
        if let Some(conn) = game.connections.get_mut(&ticket) {
            conn.dropped = true;
        }
    }
}

// carry out one control. authority over the target is checked per-control rather than up front,
// because CreateKey has no target and EndGame's target is the game itself.
fn manage(
    game: &mut GameHandle,
    caller_key: &Key,
    supervises: bool,
    control: &GameControl,
    cancel: &CancellationToken,
) -> Result<ControlResponse, ControlError> {
    match control {
        GameControl::EndGame => {
            cancel.cancel();
            Ok(ControlResponse::Ended)
        }

        GameControl::CreateKey {
            actors,
            capabilities,
        } => {
            let capabilities = to_flags(capabilities);
            if capabilities.contains(Capability::Supervise) && !supervises {
                return Err(ControlError::CannotGrantSupervise);
            }

            let key = Key::generate();
            game.keys.insert(
                key.clone(),
                KeyData {
                    // child of the game token, so teardown takes this key's connections with it.
                    cancel: game.cancel.child_token(),
                    tickets: HashSet::new(),
                    privileges: Privileges {
                        actors: actors.clone(),
                        capabilities,
                    },
                },
            );

            Ok(ControlResponse::KeyCreated { key })
        }

        GameControl::RevokeKey { key } => {
            may_manage(game, caller_key, supervises, key)?;
            revoke_key(game, key);
            Ok(ControlResponse::KeyRevoked)
        }

        GameControl::SetCapabilities { key, capabilities } => {
            may_manage(game, caller_key, supervises, key)?;

            let capabilities = to_flags(capabilities);
            // gating on the grant rather than on holding it already, so a supervisor may still strip
            // Supervise from a key that has it.
            if capabilities.contains(Capability::Supervise) && !supervises {
                return Err(ControlError::CannotGrantSupervise);
            }

            // may_manage already established the key exists.
            game.keys
                .get_mut(key)
                .expect("target key resolved by may_manage")
                .privileges
                .capabilities = capabilities;

            Ok(ControlResponse::CapabilitiesSet)
        }

        GameControl::SetActorScope { key, actors } => {
            may_manage(game, caller_key, supervises, key)?;

            game.keys
                .get_mut(key)
                .expect("target key resolved by may_manage")
                .privileges
                .actors = actors.clone();

            Ok(ControlResponse::ActorScopeSet)
        }
    }
}

// resolve the caller, gate on being an administrator at all, then hand off. the lock is held across
// the whole control so two admins cannot interleave halfway through each other's authority checks.
fn handle_control(
    state: &WrappedServerState,
    game_id: GameId,
    ticket: &Ticket,
    control: &GameControl,
    cancel: &CancellationToken,
) -> ControlOutcome {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return ControlOutcome::Denied; // game is gone
    };

    let Some(caller_key) = game.tickets.get(ticket).cloned() else {
        return ControlOutcome::Denied;
    };
    let Some(caller) = game.keys.get(&caller_key) else {
        return ControlOutcome::Denied;
    };

    // copied out so the caller's borrow ends before the target is mutated -- a caller acting on its
    // own key would otherwise be aliasing.
    let capabilities = caller.privileges.capabilities;

    if !capabilities.contains(Capability::Administer) {
        return ControlOutcome::Denied;
    }

    match manage(
        game,
        &caller_key,
        capabilities.contains(Capability::Supervise),
        control,
        cancel,
    ) {
        Ok(response) => ControlOutcome::Ok(response),
        Err(error) => ControlOutcome::Err(error),
    }
}

// fan the result of one execution out to every live connection: recipient-filtered commands for
// everyone who can see any of them, plus the request/response pair for whoever asked.
//
// `commands` is a slice of the tail of `log`, and `at` is the log position of its first element --
// visibility of a base command depends on where it sits, so the positions have to survive the trip.
//
// a connection that would see nothing and asked for nothing gets no batch and consumes no sequence
// number: seq counts what a socket was actually sent, so it must not tick for a no-op.
fn dispatch(
    state: &WrappedServerState,
    game_id: GameId,
    born: &HashMap<ActorKey, usize>,
    commands: &[CommandPayload],
    at: usize,
    reply: Option<(Ticket, ResponsePair)>,
) {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return; // game is gone, and so are its connections
    };

    // split the borrow by field: the loop holds `connections` mutably while still reading the
    // ticket/key ledger to resolve each connection's privileges.
    let GameHandle {
        tickets,
        connections,
        keys,
        ..
    } = game;

    let (reply_ticket, mut reply) = match reply {
        Some((ticket, pair)) => (Some(ticket), Some(pair)),
        None => (None, None),
    };

    for (ticket, conn) in connections.iter_mut() {
        if conn.dropped {
            continue;
        }

        // the ledger and the connection map are written together under one lock, so a live
        // connection that resolves to nothing means our own bookkeeping is inconsistent -- there is
        // no privilege set to filter against and guessing one is how you leak state.
        let Some(privileges) = tickets
            .get(ticket)
            .and_then(|key| keys.get(key))
            .map(|key_data| &key_data.privileges)
        else {
            eprintln!("connection {ticket:?} has no ledger entry -- aborting");
            std::process::abort()
        };

        let visible: Vec<CommandPayload> = commands
            .iter()
            .enumerate()
            .filter(|(offset, cmd)| can_see(privileges, born, at + offset, &cmd.recipient))
            .map(|(_, cmd)| cmd.clone())
            .collect();

        // only the originating connection gets the response, and only once.
        let response = if Some(ticket) == reply_ticket.as_ref() {
            reply.take()
        } else {
            None
        };

        if visible.is_empty() && response.is_none() {
            continue;
        }

        push_batch(
            conn,
            Batch {
                commands: visible,
                response,
            },
        );
    }
}

// tell whoever submitted an action that the engine died holding it. a server-issued action has no
// originating connection, so there is simply nobody to tell.
fn crashed(
    state: &WrappedServerState,
    game_id: GameId,
    born: &HashMap<ActorKey, usize>,
    at: usize,
    ticket: Option<Ticket>,
    request: ActionRequest,
) {
    let Some(ticket) = ticket else {
        return;
    };

    let pair = ResponsePair {
        input: ServerInput::Action(request),
        output: ExecOutcome::Action(ActionOutcome::Crashed),
    };
    dispatch(state, game_id, born, &[], at, Some((ticket, pair)));
}

// replay everything a freshly attached connection is entitled to, as its first batch.
//
// a single global log is what makes this correct in one pass: filtering preserves emission order, so
// there is no way to hand a connection an Actor command that references something a later BasePlayer
// command was supposed to create.
//
// sent even when the filtered result is empty -- it is the client's "you are caught up" marker, and
// without it a client cannot tell being up to date from not being attached yet.
fn attach(
    state: &WrappedServerState,
    game_id: GameId,
    born: &HashMap<ActorKey, usize>,
    log: &[CommandPayload],
    ticket: &Ticket,
) {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return;
    };

    // both of these absences are ordinary and must be ruled out BEFORE the ledger lookup, because
    // only a missing ledger entry under a connection that is still LIVE is a broken invariant:
    //   - gone entirely: the connection died before its attach was handled, taking its entry with it.
    //   - dropped: its key was revoked between the upgrade and now, which removes the ledger entry
    //     while the ConnHandle waits for its guard to run.
    match game.connections.get(ticket) {
        None => return,
        Some(conn) if conn.dropped => return,
        Some(_) => {}
    }

    let Some(privileges) = game.privileges(ticket) else {
        eprintln!("connection {ticket:?} has no ledger entry -- aborting");
        std::process::abort()
    };

    let commands: Vec<CommandPayload> = log
        .iter()
        .enumerate()
        .filter(|(pos, cmd)| can_see(privileges, born, *pos, &cmd.recipient))
        .map(|(_, cmd)| cmd.clone())
        .collect();

    let Some(conn) = game.connections.get_mut(ticket) else {
        return; // connection died between sending the attach event and it being handled
    };

    push_batch(
        conn,
        Batch {
            commands,
            response: None,
        },
    );
}

#[derive(Deserialize)]
struct EndGameRequest {
    platform_key: String,
}

// the platform admin's teardown path. the game admin's equivalent is GameControl::EndGame over their
// socket; both converge on cancelling the one token.
//
// this only ASKS. it does not remove the registry entry, close sockets or reap the child -- the game
// task owns all of that and does it once, on its way out, no matter which path started it. so a
// teardown that races another teardown, or a game that is already dying, needs no special handling.
async fn end_game(
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    Json(body): Json<EndGameRequest>,
) -> Result<(), ServerError> {
    if !is_platform_admin(&body.platform_key) {
        return Err(ServerError::InvalidKey);
    }

    let server_state = lock_state(&state);
    let Some(game) = server_state.games.get(&game_id) else {
        return Err(ServerError::InvalidGameId);
    };

    game.cancel.cancel();

    Ok(())
}

// permission enforcement,
// input executions,
// live client updates,
// and engine process management
async fn game(
    state: WrappedServerState,
    game_id: GameId,
    mut events: mpsc::UnboundedReceiver<GameEvent>,
    cancel: CancellationToken,
) {
    // the supervisor hands over each fresh child's pipes. unbounded rather than a size-1 channel
    // because the coordinator has to be able to TAKE ownership of the pair (the pipe halves aren't
    // Clone, so nothing that only lends a borrow works here); the coordinator drains to the newest
    // pair on wake, which is what keeps a crash-loop from feeding it a dead child's descriptors.
    let (fd_in, mut fd_out) = mpsc::unbounded_channel::<(ChildStdin, ChildStdout)>();

    // the coordinator's only way to reach the child: it holds the pipes, but the supervisor owns the
    // process. a hung engine cannot be dislodged by closing stdin -- it is not reading -- so killing
    // it has to happen over here.
    //
    // capacity 1 because the request carries no information: one pending kill says everything two
    // would, so the coordinator can try_send and drop the duplicate.
    let (kill_in, mut kill_out) = mpsc::channel::<()>(1);

    let mut process_supervisor = tokio::spawn(async move {
        loop {
            let mut child = tokio::process::Command::new(
                current_exe()
                    .expect("failed to get current exe")
                    .parent()
                    .expect("failed to get parent path")
                    .join(format!("lawliet-runtime{}", std::env::consts::EXE_SUFFIX)),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("failed to boot lawliet runtime");

            // a kill request queued against the child we just replaced would otherwise be consumed
            // below and shoot down this brand new one -- a self-sustaining crash loop. at capacity 1
            // there is at most one to throw away.
            let _ = kill_out.try_recv();

            if fd_in
                .send((child.stdin.take().unwrap(), child.stdout.take().unwrap()))
                .is_err()
            {
                break;
            }

            select! {
                _ = child.wait() => {} // died on its own
                // the coordinator declared it hung. a hang is a crash that just doesn't announce
                // itself, so the response is the same: replace the child and rebuild.
                Some(()) = kill_out.recv() => {
                    let _ = child.kill().await;
                }
            }
        }
    });

    // only one action can be processed at a time. this means that the two tasks (read + write)
    // are coupled by nature and should not be run in parallel. you cannot execute multiple things
    // at a time.
    // however, multiple inputs may be waiting in the game handle's queue at any given time.
    //
    // if receiving a new set of file descriptors while an input is in flight, and that input is an
    // engine level mechanism, it means that the input most likely triggered a crash, and so they should receive
    // an engine crashed response. if they didnt trigger a crash, it was likely some kind of freak
    // incident out of our control.
    //
    // only after the action is confirmed to be valid, and the action has been saved does the client
    // receive acknowledgement.
    //
    // commands live in ONE log in emission order; a recipient's "own log" is a filter over it. that
    // is what keeps cross-recipient order intact, so a replay can never hand out an Actor command
    // before the BasePlayer command that created what it refers to.
    //
    // sequence numbers are not saved for any given key. they are determined at runtime.
    // how it works:
    // every connection gets its own sequence number starting at 0 at runtime
    // on attach, walk the log once, keep what that connection may see, and send it as one batch --
    // which grants every actor available to that connection immediately.
    //
    // a batch consists of a command buffer and an optional structure containing both the requested
    // input and its response (if applicable).
    //
    // responses are not saved to command sequences. responses are meant only for specific connections.
    //
    // batches are sent on a best effort basis. if a client's outbox is full, the client is cut. the
    // alternatives are potentially unboundedly growing memory, or having the client lose mandatory
    // info.
    //
    // the coordinator is an asynchronous state machine
    // the coordinator takes a clone so the outer task still holds one to clean up with after aborting.
    let coordinator_cancel = cancel.clone();
    let coordinator_state = state.clone();
    let mut coordinator = tokio::spawn(async move {
        let state = coordinator_state;
        // by the time something is in flight it is necessarily an action -- controls never reach the
        // engine.
        let mut in_flight: Option<InFlight> = None;
        let mut stdin: Option<ChildStdin> = None;
        let mut stdout: Option<Lines<BufReader<ChildStdout>>> = None;

        // every action the engine accepted, in order. the source of truth for rebuilding a fresh
        // child, and what gets persisted at L1.
        let mut accepted: Vec<ActionRequest> = vec![];
        // every command ever emitted, in emission order. "per-recipient logs" are a FILTER over this,
        // not separate storage -- one log is what keeps cross-recipient order intact for free.
        let mut log: Vec<CommandPayload> = vec![];
        // where each player actor came into existence, as a position in `log`. this is what makes
        // BasePlayer resolvable: a base command belongs to actors born after it.
        let mut born: HashMap<ActorKey, usize> = HashMap::new();
        // replay responses still to be swallowed. a rebuilt child re-emits every command it already
        // emitted; `log` already holds them, so the echoes must not be logged or fanned out again.
        let mut to_discard: usize = 0;
        // watchdog: when the engine owes us a line, when we stop waiting for it. armed and disarmed
        // in one place, at the bottom of the loop.
        let mut deadline: Option<Instant> = None;
        // drawn once, here, rather than per boot: it rides in the InitializeEngine action, so a
        // rebuilt child replaying that action reproduces the same RNG stream. a fresh seed per boot
        // would make every crash silently fork the game's randomness.
        let seed = generate_seed();

        loop {
            tokio::select! {
                // crash or initial boot — resaturate the new child with everything it accepted before
                Some(fds) = fd_out.recv() => {
                    // drain to the newest pair. a crash-loop can queue several, and every pair but
                    // the last belongs to a child that is already dead -- dropping them closes those
                    // pipes, which is exactly what we want.
                    let mut fds = fds;
                    while let Ok(newer) = fd_out.try_recv() {
                        fds = newer;
                    }
                    let (new_in, new_out) = fds;

                    // an action that was in flight when the pipe died is the prime suspect for having
                    // killed it, so it is NOT added to `accepted` and never replayed. its originator
                    // is told, rather than left waiting on a reply that will never come.
                    if let Some(InFlight { ticket, request }) = in_flight.take() {
                        crashed(&state, game_id, &born, log.len(), ticket, request);
                    }

                    stdin = Some(new_in);
                    stdout = Some(BufReader::new(new_out).lines());

                    // count only what actually made it down the pipe: if the write fails partway, the
                    // remaining actions never ran, so their echoes will never arrive to be discarded.
                    let mut written = 0;
                    for request in &accepted {
                        let line = to_line(request);
                        if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                            stdin = None;
                            break;
                        }
                        written += 1;
                    }
                    to_discard = written;

                    // a game is initialized by the server, not by whoever connects first: an
                    // uninitialized engine rejects everything, so leaving it to an admin's first
                    // action would mean every game has a broken window before someone opens it.
                    //
                    // issued only when nothing has been accepted yet, which is exactly the first boot.
                    // once it succeeds it lives in `accepted` like any other action, so every
                    // subsequent rebuild replays it -- with its original seed, keeping the engine's RNG
                    // stream identical across a crash.
                    if accepted.is_empty() && stdin.is_some() {
                        let request = ActionRequest {
                            // the server's own voice. no key can ask for this (System is unreachable
                            // from a connection) and no connection is waiting on it.
                            actor: ActionActor::System,
                            timestamp: now(),
                            payload: Action::InitializeEngine(InitializeEngine { seed }),
                        };

                        let line = to_line(&request);
                        if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                            stdin = None;
                        } else {
                            in_flight = Some(InFlight { ticket: None, request });
                        }
                    }
                }

                // a connection attaching, or an input from one. gated on the engine being reachable
                // with nothing in flight, so the channel itself is the queue -- and because it is ONE
                // channel, a connection's Attach is always handled before anything it goes on to send.
                Some(event) = events.recv(), if stdin.is_some() && in_flight.is_none() => {
                    let InputEnvelope { ticket, input } = match event {
                        GameEvent::Attach { ticket } => {
                            attach(&state, game_id, &born, &log, &ticket);
                            continue;
                        }
                        GameEvent::Input(envelope) => envelope,
                    };

                    // controls are handled HERE and never forwarded: they act ON the timeline, not in
                    // the fiction, and the engine has no concept of them.
                    let request = match input {
                        ServerInput::Action(request) => request,
                        ServerInput::Control(control) => {
                            // controls carry no actor, so authority is a matter of capabilities and
                            // of the target key -- all of which lives in handle_control.
                            let outcome = handle_control(
                                &state,
                                game_id,
                                &ticket,
                                &control,
                                &coordinator_cancel,
                            );

                            // replied to even for EndGame, where the teardown races the send and will
                            // usually win. uniform beats special-casing a reply nobody is waiting on.
                            let pair = ResponsePair {
                                input: ServerInput::Control(control),
                                output: ExecOutcome::Control(outcome),
                            };
                            dispatch(&state, game_id, &born, &[], log.len(), Some((ticket, pair)));
                            continue;
                        }
                    };

                    // auth: a connection may only act as an actor its key's privilege set permits.
                    // checked here rather than at the socket because the privilege set is resolved
                    // fresh every time, so a narrowed key takes effect on its live sockets at once.
                    let permitted = {
                        let server_state = lock_state(&state);
                        server_state
                            .games
                            .get(&game_id)
                            .and_then(|game| game.privileges(&ticket))
                            .is_some_and(|privileges| privileges.can_act_as(&request.actor))
                    };

                    if !permitted {
                        let pair = ResponsePair { input: ServerInput::Action(request), output: ExecOutcome::Action(ActionOutcome::Denied) };
                        dispatch(&state, game_id, &born, &[], log.len(), Some((ticket, pair)));
                        continue;
                    }

                    let line = to_line(&request);
                    if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                        // the pipe died on the write, so this action never ran. same story as a
                        // crash; the fd arm will rebuild and resaturate.
                        //
                        // ask for the kill too. a failed write means the read end is gone, so the
                        // child is all but certainly dead already and the supervisor is on its way to
                        // replacing it -- but if it somehow is not, nothing else would ever notice:
                        // with stdin gone this arm is disabled and the watchdog is not armed, so the
                        // game would wedge in silence.
                        stdin = None;
                        let _ = kill_in.try_send(());
                        let pair = ResponsePair { input: ServerInput::Action(request), output: ExecOutcome::Action(ActionOutcome::Crashed) };
                        dispatch(&state, game_id, &born, &[], log.len(), Some((ticket, pair)));
                    } else {
                        in_flight = Some(InFlight { ticket: Some(ticket), request });
                    }
                }

                // response from child
                line = async { stdout.as_mut().unwrap().next_line().await }, if stdout.is_some() => {
                    let Ok(Some(text)) = line else {
                        stdout = None;
                        continue;
                    };

                    // the runtime is the other half of this protocol and we built both. an
                    // undeserializable line means the two binaries disagree about the wire format --
                    // a deploy mistake, not a runtime condition, and not something to limp along with.
                    let result: ExecutionResult = match serde_json::from_str(&text) {
                        Ok(result) => result,
                        Err(e) => {
                            eprintln!("engine output failed to deserialize: {e} -- aborting");
                            std::process::abort()
                        }
                    };

                    if to_discard > 0 {
                        to_discard -= 1; // resaturation echo; its commands are already in `log`
                        // the watchdog measures SILENCE, not how long the whole replay takes -- a
                        // long enough log would otherwise trip it and get a perfectly healthy child
                        // killed, then do it again on every rebuild. an echo is proof of progress, so
                        // the window restarts (and closes once nothing more is owed).
                        deadline = (to_discard > 0)
                            .then(|| Instant::now() + Duration::from_secs(ENGINE_TIMEOUT));
                        continue;
                    }

                    let Some(InFlight { ticket, request }) = in_flight.take() else {
                        // the engine spoke with nothing owed. there is no good response available:
                        // aborting punishes every other game on the box for one child's weirdness,
                        // and rebooting would just reproduce whatever caused it.
                        // TODO: log this loudly (which engine, what it said) and carry on.
                        continue;
                    };

                    let (output, commands) = match result {
                        Ok((response, context)) => {
                            accepted.push(request.clone());
                            (ActionOutcome::Ok(response), context.commands)
                        }
                        // a rejected action changed nothing of its own, so it is not replayed. it can
                        // still carry catchup commands: the job queue runs on the way in and its
                        // effects are real regardless of what the requested action did. leaving it
                        // out of `accepted` does not lose them -- catchup is driven by timestamps, so
                        // the next accepted action replays the same jobs.
                        Err((error, context)) => (ActionOutcome::Err(error), context.commands),
                    };

                    // record the actor BEFORE the commands are appended, so `born` lands on the
                    // position the new actor's own creation batch starts at -- it is entitled to the
                    // base commands that batch emits.
                    let at = log.len();
                    log.extend(commands);
                    if let ActionOutcome::Ok(ActionResponse::AddPlayer(added)) = &output {
                        // only top-level AddPlayer responses are visible from out here. a player
                        // created as a nested sub-action would need a command to announce it.
                        born.insert(added.id, log.len());
                    }

                    // commands go to everyone entitled to them either way; only the reply needs an
                    // originating connection, and a server-issued action has none.
                    let reply = ticket.map(|ticket| {
                        let pair = ResponsePair {
                            input: ServerInput::Action(request),
                            output: ExecOutcome::Action(output),
                        };
                        (ticket, pair)
                    });
                    dispatch(&state, game_id, &born, &log[at..], at, reply);
                }

                // the engine owes us a line and has not produced one. treat it exactly as a crash --
                // the only difference is that a hang never announces itself, so we have to notice.
                _ = async { sleep_until(deadline.unwrap()).await }, if deadline.is_some() => {
                    // reply here rather than leaving it to the fd arm: clearing in_flight now is what
                    // stops the watchdog re-arming and firing a second kill before the child is gone.
                    if let Some(InFlight { ticket, request }) = in_flight.take() {
                        crashed(&state, game_id, &born, log.len(), ticket, request);
                    }

                    // abandon the pipes: a late line from a child we have given up on must not be
                    // mistaken for a live response, and nothing more may be written to it.
                    stdin = None;
                    stdout = None;
                    to_discard = 0;

                    // full means a kill is already pending, which says everything this one would.
                    let _ = kill_in.try_send(());
                }

                // every branch disabled: no pipe, and no supervisor left to hand us a new one. select!
                // PANICS on that rather than parking, and a panic here dies inside a spawned task, so
                // exit deliberately instead -- nothing can make progress again either way.
                else => break,
            }

            // keep the watchdog honest: armed exactly while the engine owes us something -- a reply to
            // an in-flight action, or an outstanding replay echo. arms that `continue` skip this and
            // are responsible for their own bookkeeping (the echo branch restarts the window itself,
            // since receiving a line is progress).
            //
            // an already-running deadline is deliberately left alone. re-deriving it on every wakeup
            // would push it forward whether or not the engine did anything, which is the classic way a
            // watchdog quietly stops being one. (`sleep_until` takes an absolute instant, so rebuilding
            // the future each pass is free of that hazard -- `sleep(duration)` would restart the clock.)
            deadline = match (in_flight.is_some() || to_discard > 0, deadline) {
                (false, _) => None,
                (true, Some(running)) => Some(running),
                (true, None) => Some(Instant::now() + Duration::from_secs(ENGINE_TIMEOUT)),
            };
        }
    });

    // &mut so the handles survive the race; whichever arm wins, abort the others. aborting the
    // supervisor drops the Child it owns, and kill_on_drop reaps the engine -- so the process goes
    // even when teardown came from outside and nobody asked it to stop.
    select! {
        _ = cancel.cancelled() => {}
        _ = &mut process_supervisor => {}
        _ = &mut coordinator => {}
    }
    process_supervisor.abort();
    coordinator.abort();

    // cleanup. cancel first: this is the only path that runs it, and one of the two ways in here is a
    // task ending on its own, where nothing has cancelled anything yet. dropping a CancellationToken
    // does NOT cancel it, so removing the handle before this would strand every live socket waiting
    // on a token that will never fire, until its heartbeat eventually reaped it.
    cancel.cancel();

    // dropping the handle closes the inbox, so any connection task still on its way out finds its
    // send failing. their ClaimGuards already tolerate the game being gone.
    lock_state(&state).games.remove(&game_id);
}
