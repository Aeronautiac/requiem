// The in-memory registry: which games exist, and who is connected to them.
//
// The engine half of a game's state is rebuilt by replaying its accepted action log into a fresh
// engine child; the server's SIM state (keys, profiles) lives here and is likewise rebuilt by
// replaying the accepted stream -- every sim control is part of that stream, so a rebuild
// reconstructs the ledger from scratch. The live key handles (cancel tokens, tickets) are the one
// thing that cannot be derived and are reconciled against the rebuilt ledger.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    auth::{Key, KeyHandle, Privileges, Ticket},
    delivery::DeliveryData,
    game::GameInput,
    store::Store,
    wire::Batch,
};

pub type GameId = u64; // DB BIGSERIAL id, read back after insert. unique across restarts.

// TODO:
// there is a potential race between the outbox opening, and the initial attachment
// during a broadcast, a connection may be sent something that is NOT their initial batch before
// receiving their sync batch, but this doesn't really matter, because sync wipes client state
// anyway.
// potentially clean this up. right now it sort of works by coincidence.
pub struct ConnHandle {
    pub cancel: CancellationToken,
    pub outbox: mpsc::Sender<Batch>,
    pub dropped: bool,
    pub delivery: DeliveryData,
}

pub struct GameHandle {
    // root of this game's token tree: every key token is a child of it and every connection token a
    // child of one of those, so cancelling here reaches all of them without walking the maps.
    pub cancel: CancellationToken,
    pub inbox: mpsc::UnboundedSender<GameInput>,
    pub tickets: HashMap<Ticket, Key>,
    pub connections: HashMap<Ticket, ConnHandle>,
    // SIMULATION state: the raw authority each key holds. The authoritative copy lives in the
    // runtime; this is yagami's mirror, rebuilt from the runtime's KeyRoster outputs during a
    // rebuild (a rewind truncates the accepted stream so rolled-back keys never re-materialize).
    pub keys: HashMap<Key, Privileges>,
    // LIVE handles, outside the simulation: each key's cancel token and issued tickets. Reconciled
    // against the rebuilt `keys` ledger after every rebuild so no handle outlives (or orphans) its
    // key.
    pub key_handles: HashMap<Key, KeyHandle>,
}

impl GameHandle {
    // ticket -> key -> privilege set. the ledger holds ticket->key for the life of the connection, so
    // this resolves for as long as the connection is claimed.
    pub fn privileges(&self, ticket: &Ticket) -> Option<&Privileges> {
        let key = self.tickets.get(ticket)?;
        self.keys.get(key)
    }
}

pub struct ServerState {
    pub store: Store, // set once at startup; both handlers and game tasks read it here
    pub games: HashMap<GameId, GameHandle>,
}
pub type WrappedServerState = Arc<Mutex<ServerState>>;

// insert a GameHandle into the registry synchronously. used by boot-time resume (main), which
// registers every active game before axum starts serving -- so there is no window where a
// restarted game is in the DB but unreachable. a fresh game registers itself (via its task) after
// its first boot succeeds. `keys` preloads the handle's key ledger so get_ticket can validate keys
// before the task finishes its boot replay -- what makes on-demand (rather than immediate) boot
// possible.
pub fn insert_handle(
    state: &WrappedServerState,
    game_id: GameId,
    inbox: mpsc::UnboundedSender<GameInput>,
    cancel: CancellationToken,
    keys: HashMap<Key, Privileges>,
) {
    let mut server_state = lock_state(state);
    server_state.games.insert(
        game_id,
        GameHandle {
            cancel,
            inbox,
            tickets: HashMap::new(),
            connections: HashMap::new(),
            keys,
            key_handles: HashMap::new(),
        },
    );
}

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
