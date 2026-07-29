// The in-memory registry: which games exist, and who is connected to them.
//
// None of this is durable. Rebuilding a game replays its action log into a fresh engine child;
// keys and connections have no such log and are simply lost on restart. That is the gap
// persistence will have to close, and it is a different shape from the engine's -- a mutable
// record, not an append-only one.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use lawliet_types::common::ActorKey;

use crate::{
    auth::{Key, KeyData, Privileges, Ticket},
    delivery::ViewportCursor,
    game::GameEvent,
    wire::{Profile, ServerOutput},
};

pub type GameId = u64; // for now, strictly incrementing

pub struct ConnHandle {
    pub cancel: CancellationToken,
    pub outbox: mpsc::Sender<ServerOutput>,
    // set when the game task cuts this connection; the connection task hasn't torn down yet. fan-out
    // skips a dropped entry in the window between the cancel and the ClaimGuard actually removing it.
    pub dropped: bool,
    // this connection's own sequence counter. per-connection and runtime-only: it counts batches
    // THIS socket was sent, so it is dense with no gaps, which is what lets the client treat a gap as
    // a desync. 0 means nothing sent yet; the first batch is 1.
    pub seq_num: u64,
    // how far this connection has been delivered, per viewport. None until it attaches: a
    // connection is in this map from the moment its socket is claimed, which is before the game
    // task has replayed anything to it, and advancing a cursor from a baseline it was never given
    // would leave it permanently short of the history that baseline assumes.
    //
    // fan-out therefore skips a connection with no cursor entirely. it loses nothing: attach
    // replays from position 0 and hands it everything, including whatever was emitted during the
    // window.
    pub cursor: Option<ViewportCursor>,
}

pub struct GameHandle {
    // root of this game's token tree: every key token is a child of it and every connection token a
    // child of one of those, so cancelling here reaches all of them without walking the maps.
    pub cancel: CancellationToken,
    pub inbox: mpsc::UnboundedSender<GameEvent>,
    pub tickets: HashMap<Ticket, Key>,
    pub connections: HashMap<Ticket, ConnHandle>,
    pub keys: HashMap<Key, KeyData>,
    // actor -> what the SERVER knows about whoever is playing that slot. The engine's MapActor says
    // the slot exists; this says who is on it, and the two have different lifetimes -- a name can be
    // set long after the slot, and changed again later. Runtime-only like the rest of this file.
    //
    // An entry here is NOT permission to see it: a profile only ever goes to a connection that has
    // already been delivered that actor's MapActor. See wire::ProfileUpdate.
    pub profiles: HashMap<ActorKey, Profile>,
}

impl GameHandle {
    // ticket -> key -> privilege set. the ledger holds ticket->key for the life of the connection, so
    // this resolves for as long as the connection is claimed.
    pub fn privileges(&self, ticket: &Ticket) -> Option<&Privileges> {
        let key = self.tickets.get(ticket)?;
        Some(&self.keys.get(key)?.privileges)
    }
}

#[derive(Default)]
pub struct ServerState {
    pub next_game_id: GameId,
    pub games: HashMap<GameId, GameHandle>,
}
pub type WrappedServerState = Arc<Mutex<ServerState>>;

// a poisoned lock means a thread panicked mid-mutation, so the maps can no longer be trusted. that
// is a process-wide problem: take the process down and let the supervisor restart us.
//
// deliberately not unwrap(): a panic here would be caught at the tokio task boundary, killing one
// task while leaving the poisoned state in place and the server running. abort is the only response
// that is loud and deterministic regardless of whether we are already unwinding.
pub fn lock_state(state: &Mutex<ServerState>) -> MutexGuard<'_, ServerState> {
    state.lock().unwrap_or_else(|_| {
        eprintln!("server state mutex poisoned -- aborting");
        std::process::abort()
    })
}
