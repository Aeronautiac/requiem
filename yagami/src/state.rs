// The in-memory registry: which games are running, and who is connected to them.
//
// The engine half of a game's state is rebuilt by replaying its accepted action log into a fresh
// engine child; the server's SIM state (keys, profiles) lives here and is likewise rebuilt by
// replaying the accepted stream -- every sim control is part of that stream, so a rebuild
// reconstructs the ledger from scratch. The live key handles (cancel tokens, tickets) are the one
// thing that cannot be derived and are reconciled against the rebuilt ledger.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use governor::Quota;
use tokio::sync::{mpsc, watch};
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

// a RUNNING game. a hibernated game has no handle at all -- only its DB row -- and get_ticket wakes
// it by registering a fresh one (see game::wake).
pub struct GameHandle {
    // root of this game's token tree: every key token is a child of it and every connection token a
    // child of one of those, so cancelling here reaches all of them without walking the maps.
    pub cancel: CancellationToken,
    pub inbox: mpsc::UnboundedSender<GameInput>,
    // every ticket issued, claimed or not -- so an empty map means nobody is on the game or about to
    // be, which is what lets it hibernate.
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
    // the handle is on its way out (see close). get_ticket waits on `gone` instead of issuing
    // against it, then starts over and finds the game asleep (or ended).
    pub closing: bool,
    // never sent on; its drop is the signal. the sender dies with this handle, so a receiver's
    // changed() resolves exactly when the handle has left the registry -- unlike `cancel`, which
    // stops the task and fires BEFORE removal on every path but the task's Drop.
    pub gone: watch::Sender<()>,
}

impl GameHandle {
    // the one way a running game stops -- hibernating, ending, or tearing down after a failure. no
    // further ticket is issued against the handle, and the cancel ends the task's loop and closes
    // every connection. the handle itself leaves the registry when the task drops.
    pub fn close(&mut self) {
        self.closing = true;
        self.cancel.cancel();
    }

    // ticket -> key -> privilege set. the ledger holds ticket->key for the life of the connection, so
    // this resolves for as long as the connection is claimed.
    pub fn privileges(&self, ticket: &Ticket) -> Option<&Privileges> {
        let key = self.tickets.get(ticket)?;
        self.keys.get(key)
    }
}

// how this deployment runs, from the environment (see http::Config). Copy, so a handler can take it
// out from under the lock instead of borrowing it alongside `games`.
#[derive(Clone, Copy)]
pub struct Settings {
    // the most games that may run at once.
    pub max_resident: usize,
    // how long a game goes without a connection or an outstanding ticket before it hibernates.
    pub hibernate_after: Duration,
    // the gap between null ticks, which advance the game's clock while nobody acts.
    pub null_tick_interval: Duration,
    // how long the runtime child gets to take or answer one input before it is treated as hung.
    pub engine_timeout: Duration,
    // boot retries back off exponentially from this delay (base, 2x, 4x, ...).
    pub boot_retry_base: Duration,
    // after this many consecutive failed boots a game gives up and tears itself down (a fresh game
    // is reported as failed and never written to the DB) rather than retrying forever.
    pub boot_max_retries: u32,
    // how long a game whose boot gave up is refused a wake, so a broken game is not re-booted by
    // every client retry.
    pub boot_cooldown: Duration,
    // tickets a key may hold at once, claimed or not -- and so its most simultaneous connections.
    pub ticket_limit: usize,
    // how long an issued ticket waits to be claimed before it is dropped.
    pub ticket_timeout: Duration,
    // batches a connection's outbox holds before it counts as too slow and is dropped.
    pub outbox_size: usize,
    // the most commands sent in one websocket message; larger batches are split.
    pub batch_size: usize,
    // how often the server pings each connection.
    pub heartbeat_interval: Duration,
    // how long a connection may go without sending any frame before it is reaped as dead.
    pub heartbeat_timeout: Duration,
    // the largest websocket message a client may send. every input is one small action, so this is
    // only a transport ceiling; a message over it closes the socket.
    pub max_input_bytes: usize,
    // the largest REST request body. every body is a token or two.
    pub max_body_bytes: usize,
    // each connection's input budget. per connection, not per key: several people may share a key.
    // a connection over it is not rejected: it stops reading until a token frees up, and TCP pushes
    // the backlog back onto the sender.
    pub input_quota: Quota,
    // each client IP's REST budget. over it is answered RateLimited.
    pub request_quota: Quota,
}

pub struct ServerState {
    pub store: Store, // set once at startup; both handlers and game tasks read it here
    pub games: HashMap<GameId, GameHandle>, // running games only
    // fresh games still booting toward registration. each holds a running slot from create_game
    // until its task registers the handle or dies.
    pub creating: usize,
    // games whose boot gave up, and when. refused a wake until the cooldown passes.
    pub boot_failures: HashMap<GameId, Instant>,
    pub settings: Settings,
}
pub type WrappedServerState = Arc<Mutex<ServerState>>;

impl ServerState {
    // may no further game start running?
    pub fn at_capacity(&self) -> bool {
        self.games.len() + self.creating >= self.settings.max_resident
    }

    pub fn record_boot_failure(&mut self, game_id: GameId) {
        self.boot_failures.insert(game_id, Instant::now());
    }

    // did this game's boot give up too recently to try again? expired failures are dropped here, so
    // the map only ever holds live cooldowns.
    pub fn boot_cooling_down(&mut self, game_id: GameId) -> bool {
        let cooldown = self.settings.boot_cooldown;
        self.boot_failures
            .retain(|_, failed_at| failed_at.elapsed() < cooldown);
        self.boot_failures.contains_key(&game_id)
    }
}

// register a running game's handle, under the caller's lock. a fresh game registers itself (via its
// task) after its first boot succeeds; a hibernated game is registered by game::wake before its task
// has booted. `keys` preloads the key ledger and each key gets its live handle here, so get_ticket
// can issue tickets while the boot replay is still running -- connections queue behind it.
pub fn insert_handle(
    server_state: &mut ServerState,
    game_id: GameId,
    inbox: mpsc::UnboundedSender<GameInput>,
    cancel: CancellationToken,
    keys: HashMap<Key, Privileges>,
) {
    let key_handles = keys
        .keys()
        .map(|key| {
            let handle = KeyHandle {
                cancel: cancel.child_token(),
                tickets: HashSet::new(),
            };
            (key.clone(), handle)
        })
        .collect();
    server_state.games.insert(
        game_id,
        GameHandle {
            cancel,
            inbox,
            tickets: HashMap::new(),
            connections: HashMap::new(),
            keys,
            key_handles,
            closing: false,
            gone: watch::Sender::new(()),
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
