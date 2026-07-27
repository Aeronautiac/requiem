// The axum surface: configuration, the REST endpoints, and the websocket upgrade plus its pump.
//
// Everything here is edge work -- parsing, authenticating, and handing off. Once a socket is live
// its traffic belongs to the game task, and this module only shuttles bytes between the two.

use std::{
    collections::{HashMap, HashSet},
    env,
    net::SocketAddr,
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
use serde::{Deserialize, Serialize};
use tokio::{
    select,
    sync::mpsc::{self},
    time::{interval, sleep, timeout},
};
use tokio_util::sync::CancellationToken;

use crate::{
    auth::{ActorScope, Capability, Key, KeyData, Privileges, Ticket},
    constants::{
        HEARTBEAT_INTERVAL, HEARTBEAT_TIMEOUT, OUTBOX_BUF_SIZE, TICKET_LIMIT, TICKET_TIMEOUT,
    },
    game::{GameEvent, InputEnvelope, game},
    now,
    state::{ConnHandle, GameHandle, GameId, WrappedServerState, lock_state},
    wire::{ServerInput, ServerOutput},
};

pub fn req(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("missing required env var: {key}"))
}

pub struct Config {
    pub bind_addr: SocketAddr,
    pub allowed_origin: HeaderValue,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
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
pub enum ServerError {
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

#[derive(Deserialize)]
pub struct TicketRequest {
    key: Key,
}

pub async fn get_ticket(
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
pub async fn establish_ws_connection(
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
            // installed by attach, once the game task has replayed the log to this socket.
            cursor: None,
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
pub struct CreateGame {
    platform_key: String, // these are strings because they are created explicitly by a platform admin
}

#[derive(Serialize)]
pub struct GameCreationPacket {
    game_id: GameId,
    admin_key: Key,
}

// who may create and tear down games at all. server-wide, and one level ABOVE per-game keys: a game
// knows only its own keys, never this. kept a seam so swapping the placeholder for real accounts
// never touches the key model.
pub fn is_platform_admin(_platform_key: &str) -> bool {
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
pub async fn create_game(
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
                profiles: HashMap::new(),
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
