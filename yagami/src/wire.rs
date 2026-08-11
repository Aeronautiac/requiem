use lawliet_types::{
    action::{ActionError, ActionRequest, ActionResponse},
    command::Command,
    common::{ActorKey, Time, ViewportKey, ID},
};
use serde::{Deserialize, Serialize};

use crate::auth::{from_flags, ActorScope, Capability, Key};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Profile {
    pub display_name: Option<String>,
}

// client view of a set of privileges
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct PrivilegeSet {
    pub actors: ActorScope,
    pub capabilities: Vec<Capability>,
}

#[derive(Serialize)]
pub enum ControlError {
    KeyNotFound,
    CannotActOnSelf,
    RequiresSupervise,
    CannotGrantSupervise,
}

#[derive(Serialize)]
pub enum ControlResponse {
    KeyCreated { key: Key },
    KeyRevoked,
    CapabilitiesSet,
    ActorScopeSet,
    ProfileSet,
}

// Every server output is either a response to some input, or the result of some internal process.
// An output can only respond to ONE input, and an input can only be attributed to ONE connection.

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub enum ViewGate {
    // connection-wide: this output is addressed to the connection itself, not to a view. passes for
    // every connection; the client reads it directly (its own privileges) rather than routing it.
    Connection,
    Admin,
    Viewport(ViewportKey), // must be in this viewport
    Player(ActorKey),      // must have access to this actor
}

// engine recipient to server view gate:
// directed to system? privilege administers.
// directed to viewport? viewport.
// directed to actor? player.

#[derive(Serialize)]
pub enum ActionOutcome {
    Ok(ActionResponse),
    Err(ActionError),
    Denied,
    EnginePanic,
}

#[derive(Serialize)]
pub enum ControlOutcome {
    Ok(ControlResponse),
    Err(ControlError),
    Denied,
}

// either a response or an error
#[derive(Serialize)]
pub enum ExecOutcome {
    Action(ActionOutcome),
    Control(ControlOutcome),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum AdminControl {
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
    GoToTime {
        time: Time,
    },
}

#[derive(Serialize, Deserialize)]
pub enum ServerInput {
    Action(ActionRequest),
    Control(AdminControl),
}

#[derive(Serialize)]
pub struct ResponsePair {
    pub response: ExecOutcome,
    pub input: ServerInput,
}

// For abilities like autopsy and tap in which request the server to output filtered data that
// cannot be stored on the engine
#[derive(Serialize, Clone, Debug)]
pub struct LogCommand {
    pub time: Time,
    pub data: Command,
}

#[derive(Serialize, Clone, Debug)]
pub enum LogType {
    Autopsy(ActorKey),
    TapIn(ID),
}

#[derive(Serialize, Clone, Debug)]
pub enum ServerCmd {
    LogDump {
        data: Vec<LogCommand>,
        log_type: LogType,
    },
    ProfileRoster {
        profiles: Vec<(ActorKey, Profile)>,
    },
    KeyRoster {
        keys: Vec<(Key, PrivilegeSet)>,
    },
    // What THIS connection's own key permits. Sent as the first command of every sync -- a fresh
    // attach or a re-sync after a privilege change -- so a client can render its own standing
    // before any gated output arrives, and again whenever that standing changes.
    Privileges(PrivilegeSet),
}

// The auth layer keeps capabilities as a bitmask (for the game task's fast permission checks);
// the wire carries an enumerated set of names so a hand-written client never has to know bit
// values.
pub fn privileges_to_wire(privileges: &crate::auth::Privileges) -> PrivilegeSet {
    PrivilegeSet {
        actors: privileges.actors.clone(),
        capabilities: from_flags(privileges.capabilities),
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum OutputData {
    Engine(Command),
    Server(ServerCmd),
}

#[derive(Serialize, Clone, Debug)]
pub struct ServerOutput {
    pub time: Time,
    // if at least one gate passes, a client may receive this output
    pub view_gates: Vec<ViewGate>,
    pub data: OutputData,
}

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
    // outputs are sent to everyone who passes the view gate
    pub outputs: Vec<ServerOutput>,
}
