// yagami-wire: the shared sim + wire types between yagami (the server) and yagami-runtime (the
// simulation child process), and the shapes amane's hand-written bindings.ts mirrors.
//
// Two concerns live here:
//   * CREDENTIAL/SIM types -- Key, Capability, ActorScope, Privileges. The unit of authorization is
//     the KEY: a key resolves to a privilege set, and every question is "does this set permit X".
//     There is no admin-vs-player key type. These are the simulation's raw authority data.
//   * WIRE types -- what goes over the socket (server -> client) and over the runtime pipe. An
//     output can only respond to ONE input, and an input can only be attributed to ONE connection.

use std::collections::HashSet;
use std::fmt::Write as _;

use enumflags2::{BitFlags, bitflags};
use lawliet_types::{
    action::{ActionActor, ActionError, ActionRequest, ActionResponse},
    command::Command,
    common::{ActorKey, ID, LogID, Time, ViewportKey},
};
use serde::{Deserialize, Serialize};

// ===== CREDENTIAL / SIM ===== //

pub type Token = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key(Token);

impl Key {
    // construct from a raw token. used by the runtime's deterministic key generation (the sim RNG).
    pub fn from_token(token: Token) -> Self {
        Self(token)
    }
}

pub fn generate_token() -> Token {
    let mut bytes = [0u8; 32]; // 256 bits of entropy
    getrandom::fill(&mut bytes).expect("OS CSPRNG unavailable");

    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

// a capability is a thing a key may do that isn't "act as an actor". there is deliberately NO
// admin-vs-player key TYPE -- "admin" is just the key whose privilege set is maximal. the game only
// ever asks "does this set permit X". adding capabilities is additive.
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
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
pub type Capabilities = BitFlags<Capability>;

// the wire carries a list of names rather than a bitmask so a hand-written client never has to know
// bit values; BitFlags is the in-memory representation only.
pub fn to_flags(capabilities: &[Capability]) -> Capabilities {
    capabilities
        .iter()
        .fold(BitFlags::empty(), |flags, capability| flags | *capability)
}

pub fn from_flags(capabilities: Capabilities) -> Vec<Capability> {
    capabilities.iter().collect()
}

// which actors a key may act as / observe. `All` is not the same as a set holding every actor that
// exists today: it covers actors created LATER, so an admin key needs no bookkeeping when the engine
// adds a player. an `Only` set is enumerated at mint time and never has to be topped up.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ActorScope {
    All,
    Only(HashSet<ActorKey>),
}

impl ActorScope {
    pub fn contains(&self, actor: &ActorKey) -> bool {
        match self {
            Self::All => true,
            Self::Only(actors) => actors.contains(actor),
        }
    }
}

// what a key is allowed to do. resolved from a ticket at the moment of use and never copied into the
// connection -- so narrowing or revoking a key takes effect on its live sockets immediately.
//
// Clone because a privilege CHANGE has to carry the previous set to the game task: widening is
// delivered as the difference between the two, and by the time the task handles it the ledger holds
// only the new one.
#[derive(Serialize, Clone)]
pub struct Privileges {
    pub actors: ActorScope,
    pub capabilities: Capabilities,
}

impl Privileges {
    // Administer is asked about often enough, and by enough different code, to be worth a name.
    pub fn administers(&self) -> bool {
        self.capabilities.contains(Capability::Administer)
    }

    // may a connection holding this set submit an action as this actor?
    pub fn can_act_as(&self, actor: &ActionActor) -> bool {
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

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Profile {
    pub display_name: Option<String>,
}

// client view of a set of privileges
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PrivilegeSet {
    pub actors: ActorScope,
    pub capabilities: Vec<Capability>,
}

// The auth layer keeps capabilities as a bitmask (for fast permission checks); the wire carries an
// enumerated set of names so a hand-written client never has to know bit values.
pub fn privileges_to_wire(privileges: &Privileges) -> PrivilegeSet {
    PrivilegeSet {
        actors: privileges.actors.clone(),
        capabilities: from_flags(privileges.capabilities),
    }
}

// ===== WIRE: controls, inputs, outcomes ===== //

#[derive(Serialize, Deserialize, Clone)]
pub enum ControlError {
    KeyNotFound,
    CannotActOnSelf,
    RequiresSupervise,
    CannotGrantSupervise,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ControlResponse {
    KeyCreated { key: Key },
    KeyRevoked,
    CapabilitiesSet,
    ActorScopeSet,
    ProfileSet,
    ReSeed,
    TimeSet,
    // the engine's input version, in reply to a server-issued GetVersion query.
    EngineVersion(u64),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ActionOutcome {
    Ok(ActionResponse),
    Err(ActionError),
    Denied,
    EnginePanic,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ControlOutcome {
    Ok(ControlResponse),
    Err(ControlError),
    Denied,
}

// either a response or an error
#[derive(Serialize, Deserialize, Clone)]
pub enum ExecOutcome {
    Action(ActionOutcome),
    Control(ControlOutcome),
}

// A simulation-level control: it mutates the server's sim state (the keys/profiles ledger) and is
// part of the accepted stream, so a rebuild replays it to reconstruct that state. It carries the
// game time it was applied at -- the same override the game task stamps on engine actions -- so a
// time travel rewind that truncates the accepted stream drops the sim controls of the invalidated
// future along with the engine actions.
#[derive(Serialize, Deserialize, Clone)]
pub struct SimControl {
    pub time: Time,
    pub data: SimControlData,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum SimControlData {
    // the key is generated by the runtime from its seeded simulation RNG -- never sent on the wire.
    // a rebuild replays the stream in the same order, so the same key is regenerated at the same
    // position, deterministically.
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
    SetProfile {
        actor: ActorKey,
        profile: Profile,
    },
    // rotate the simulation RNG to a fresh seed. server-issued: the game task injects one after a
    // name leaves the server's secrecy (a true name is revealed or a display roster is broadcast),
    // so a name learned now cannot predict the ones drawn after it -- each epoch is independent.
    // part of the accepted stream, so a rebuild replays the same rotations and reproduces the names.
    ReSeed {
        seed: u64,
    },
    // a server-issued query for the engine's input version (Engine::version). NOT part of the
    // accepted stream: it changes nothing, so it is never persisted or replayed. the game task
    // dispatches it once per boot to learn what version to stamp on subsequently-accepted inputs.
    GetVersion,
}

// A meta-level control acts on the timeline itself rather than the sim, so it is NOT part of the
// accepted stream (and needs no sim time). GoToTime is the game task's own time-travel mechanic.
#[derive(Serialize, Deserialize, Clone)]
pub enum MetaControl {
    GoToTime { time: Time },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum AdminControl {
    Sim(SimControl),
    Meta(MetaControl),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ServerInput {
    Action(ActionRequest),
    Control(AdminControl),
}

// The unit the accepted stream is persisted and replayed in: the input plus the engine version that
// interpreted it. The server stamps `version` (learned once per boot by querying the runtime for
// the engine's version) when it first accepts an input, so a later engine bugfix can tell, per
// input, which semantics were in force and migrate a replay without corrupting the timeline. The
// client->server wire still carries a bare ServerInput; this wrap exists only where the stream is
// stored, and its version rides the runtime pipe so the engine can execute under the input's own
// semantics. Only the client never sees it.
#[derive(Serialize, Deserialize, Clone)]
pub struct VersionedInput {
    pub version: u64,
    pub input: ServerInput,
}

#[derive(Serialize, Deserialize)]
pub struct ResponsePair {
    pub response: ExecOutcome,
    pub input: ServerInput,
}

// ===== WIRE: the output stream ===== //
//
// history holds ONE type of element: `Output`. its `recipients` field is BOTH the audience check
// (who may see it) AND the partition key (which viewport/log it belongs to) -- never folded away.
// `Log` is the recipient `ViewGate` used to lack: a record kept for later dump queries, delivered to
// no client live. The runtime owns the data viewport and the engine's recipient, so it stamps
// `recipients` for engine commands and sim outputs alike; yagami stamps them for its own `Server`
// outputs (timeline, sync, time anchor).

// who an output is for -- the audience check AND the partition key, unified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Recipient {
    Admin,
    Viewport(ViewportKey), // must be in this viewport (and indexed for backfill)
    Player(ActorKey),      // must have access to this actor
    Log(LogID),            // kept for later dump queries, delivered to no client live
}

// engine recipient -> wire recipient:
// directed to system? admin. directed to viewport? viewport (+ admin). directed to actor? player.
// directed to log? log. a record addressed to Log reaches nobody live but is indexed for dumps.

// For abilities like autopsy and tap in which request the server to output filtered data that
// cannot be stored on the engine
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogCommand {
    pub time: Time,
    pub data: Command,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LogType {
    Autopsy(ActorKey),
    TapIn(ID),
}

// a sim-level projection of server state, emitted by the runtime when sim state changes. kept as
// rosters for now (room to simplify later). the runtime addresses them via `recipients`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SimOutput {
    ProfileRoster { profiles: Vec<(ActorKey, Profile)> },
    KeyRoster { keys: Vec<(Key, PrivilegeSet)> },
    LogAction { action: ActionRequest },
}

// yagami-level concerns the runtime never produces: LogAction (admin timeline), LogDump (the
// delivery-time transform of a Log-recipient engine command), Privileges (per-connection sync),
// GameClock (time anchor).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerCmd {
    LogDump {
        data: Vec<LogCommand>,
        log_type: LogType,
    },
    // What THIS connection's own key permits. Sent as the first command of every sync -- a fresh
    // attach or a re-sync after a privilege change -- so a client can render its own standing
    // before any gated output arrives, and again whenever that standing changes.
    Privileges(PrivilegeSet),
    // anchor the client's game time
    GameClock {
        sent_at: u128, // real world time
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OutputData {
    // an engine command, straight from lawliet.
    Engine(Command),
    // a sim-state projection from the runtime (keys/profiles ledger).
    Sim(SimOutput),
    // a yagami-level concern the runtime never produces.
    Server(ServerCmd),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub time: Time, // game time
    // if at least one recipient passes, a client may see this output. also the partition key.
    pub recipients: Vec<Recipient>,
    pub data: OutputData,
}

// ===== WIRE: batches (delivery-level; yagami-only) ===== //

// a batch can either be live, or it can be an initial batch which tells a client to initialize their
// state, or reset their current state and construct a new state with the initialize batch.
// widening?
// widening becomes a rescan under the actor's new permissions, and a re-initialization.
// this is slightly inefficient, but a widening barely occurs, and it's correct.
// a narrowing too, however it should be noted that you cannot truly get rid of that data given a
// client that doesnt comply to the protocol. as soon as a client is given permissions, you
// should assume that they have everything. all a narrowing does is prevent them from acting or
// receiving new data from those permissions.
// what about backward time travel?
// everyone is reinitialized, some are kicked if their key was invalidated.
#[derive(Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum BatchKind {
    Live(Option<ResponsePair>),
    Initialize,
}

// clients receive filtered batches rather than many disconnected events
#[derive(Serialize)]
pub struct Batch {
    // the response pair is sent only to the connection which triggered the batch
    pub kind: BatchKind,
    // outputs are sent to everyone who passes a recipient
    pub outputs: Vec<Output>,
}
