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

use lawliet_types::common::{ActorKey, Time};

use crate::{
    auth::{Key, KeyData, Privileges, Ticket},
    delivery::DeliveryData,
    game::GameInput,
    wire::{Batch, Profile},
};

pub type GameId = u64; // for now, strictly incrementing

pub struct ConnHandle {
    pub cancel: CancellationToken,
    pub outbox: mpsc::Sender<Batch>,
    // set when the game task cuts this connection; the connection task hasn't torn down yet. fan-out
    // skips a dropped entry in the window between the cancel and the ClaimGuard actually removing it.
    pub dropped: bool,
    // how far this connection has been delivered, per viewport, and which actors still grant access.
    // filled by History's delivery as it walks the log for this connection.
    pub delivery: DeliveryData,
}

pub struct GameHandle {
    // root of this game's token tree: every key token is a child of it and every connection token a
    // child of one of those, so cancelling here reaches all of them without walking the maps.
    pub cancel: CancellationToken,
    pub inbox: mpsc::UnboundedSender<GameInput>,
    pub tickets: HashMap<Ticket, Key>,
    pub connections: HashMap<Ticket, ConnHandle>,
    pub keys: HashMap<Key, KeyData>,
    // actor -> what the SERVER knows about whoever is playing that slot. The engine's MapActor says
    // the slot exists; this says who is on it, and the two have different lifetimes -- a name can be
    // set long after the slot, and changed again later. Runtime-only like the rest of this file.
    //
    // An entry here is NOT permission to see it: a profile only ever goes to a connection that has
    // already been delivered that actor's MapActor.
    pub profiles: HashMap<ActorKey, Profile>,
    // timeline: when each actor slot was mapped, and when each key/profile was minted. kept so a
    // rewind can discard server-side state that stands on an actor or moment that no longer exists.
    pub actor_created: HashMap<ActorKey, Time>,
    pub key_created: HashMap<Key, Time>,
    pub profile_created: HashMap<ActorKey, Time>,
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
