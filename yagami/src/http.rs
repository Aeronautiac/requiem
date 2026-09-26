// The axum surface: configuration, the REST endpoints, and the websocket upgrade plus its pump.
//
// Everything here is edge work -- parsing, authenticating, and handing off. Once a socket is live
// its traffic belongs to the game task, and this module only shuttles bytes between the two.

use std::{
    env, fmt::Display, future::ready, net::SocketAddr, num::NonZeroU32, str::FromStr,
    time::Duration,
};

use axum::{
    Json,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderValue, StatusCode},
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

use crate::{
    auth::{ActorScope, Capability, Key, KeyHandle, Ticket},
    delivery::DeliveryData,
    game::{GameCommand, GameInput, GameStart, InputEnvelope, game, wake},
    state::{ConnHandle, GameHandle, GameId, Settings, WrappedServerState, lock_state},
    store::Store,
    wire::{
        AdminControl, Batch, ControlOutcome, ControlResponse, ExecOutcome, ServerInput, SimControl,
        SimControlData,
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
        };
        (status, Json(self)).into_response()
    }
}

#[derive(Deserialize)]
pub struct TicketRequest {
    key: Key,
}

// a running game answers from its handle. a hibernated one has no handle: its key is checked
// against the durable ledger first -- so no key can wake a game it does not belong to -- and then
// the game is woken and the ticket issued against its fresh handle. a game caught mid-hibernation is
// waited out and then woken, so the caller never sees the difference.
pub async fn get_ticket(
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    Json(body): Json<TicketRequest>,
) -> Result<Ticket, ServerError> {
    let key = body.key;
    loop {
        // subscribed under the lock, while the handle still exists, so its removal cannot be missed.
        let closing = {
            let mut server_state = lock_state(&state);
            let settings = server_state.settings;
            match server_state.games.get_mut(&game_id) {
                Some(game_state) if game_state.closing => Some(game_state.gone.subscribe()),
                Some(game_state) => {
                    return issue_ticket(&state, settings, game_state, game_id, key);
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

        let store = lock_state(&state).store.clone();
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

        let mut server_state = lock_state(&state);
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
        wake(&state, &mut server_state, game_id, keys);
        let settings = server_state.settings;
        let game_state = server_state
            .games
            .get_mut(&game_id)
            .expect("just woken, under this lock");
        return issue_ticket(&state, settings, game_state, game_id, key);
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

#[derive(Deserialize)]
pub struct CreateGame {
    platform_key: String, // these are strings because they are created explicitly by a platform admin
}

#[derive(Serialize)]
pub struct GameCreationPacket {
    game_id: GameId,
    admin_key: Key,
}

// a platform admin is not a game admin. this gives you access to PLATFORM CONTROLS like creating
// and killing games. the allowlist lives in the `platform_keys` table, editable in the DB UI.
async fn is_platform_admin(store: &Store, platform_key: &str) -> Result<bool, ServerError> {
    store
        .is_platform_admin(platform_key)
        .await
        .map_err(|_| ServerError::InvalidKey)
}

// to create a game, you must have a platform key
// returns the id of the game created and the admin key
// must create the game entry in the REST endpoint, but cleanup can be handled outside of it (games
// are guaranteed to be created after auth, so there is no failure case with a weird cleanup scenario).
pub async fn create_game(
    State(state): State<WrappedServerState>,
    Json(body): Json<CreateGame>,
) -> Result<Json<GameCreationPacket>, ServerError> {
    let store = lock_state(&state).store.clone();
    if !is_platform_admin(&store, &body.platform_key).await? {
        return Err(ServerError::InvalidKey);
    }
    // reserve the new game's running slot. its task holds the reservation until it registers or
    // dies, and nothing awaits between here and the spawn below, so the slot always has an owner.
    {
        let mut server_state = lock_state(&state);
        if server_state.at_capacity() {
            return Err(ServerError::ServerAtCapacity);
        }
        server_state.creating += 1;
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

    let admin_key = responses.into_iter().find_map(|outcome| match outcome {
        ExecOutcome::Control(ControlOutcome::Ok(ControlResponse::KeyCreated { key })) => Some(key),
        _ => None,
    });

    match admin_key {
        Some(admin_key) => Ok(Json(GameCreationPacket { game_id, admin_key })),
        None => Err(ServerError::GameBootFailed),
    }
}

#[derive(Deserialize)]
pub struct EndGameRequest {
    platform_key: String,
}

// the platform admin's teardown path. the game admin's equivalent is GameControl::EndGame over their
// socket; both converge on cancelling the one token.
//
// this only ASKS. it does not remove the registry entry, close sockets or reap the child -- the game
// task owns all of that and does it once, on its way out, no matter which path started it. so a
// teardown that races another teardown, or a game that is already dying, needs no special handling.
pub async fn end_game(
    State(state): State<WrappedServerState>,
    Path(game_id): Path<GameId>,
    Json(body): Json<EndGameRequest>,
) -> Result<(), ServerError> {
    let store = lock_state(&state).store.clone();
    if !is_platform_admin(&store, &body.platform_key).await? {
        return Err(ServerError::InvalidKey);
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
