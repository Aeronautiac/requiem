use lawliet_types::{
    action::{ActionError, ActionRequest, ActionResponse},
    command::Command,
    common::{ActorKey, ID, Time, ViewportKey},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{ActorScope, Capability, Key},
    wire::{ControlError, ControlResponse, PrivilegeSet, Profile},
};

// Every server output is either a response to some input, or the result of some internal process.
// An output can only respond to ONE input, and an input can only be attributed to ONE connection.

#[derive(Serialize)]
pub enum ViewGate {
    Privileges(PrivilegeSet), // must have all of these privileges
    Viewport(ViewportKey),    // must be in this viewport
    Player(ActorKey),         // must have access to this actor
}

#[derive(Serialize)]
pub enum ActionOutcome {
    Ok(ActionResponse),
    Err(ActionError),
    Denied,
    Crashed,
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

#[derive(Serialize, Deserialize)]
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
#[derive(Serialize)]
pub struct LogCommand {
    pub time: Time,
    pub data: Command,
}

#[derive(Serialize)]
pub enum LogType {
    Autopsy(ActorKey),
    TapIn(ID),
}

#[derive(Serialize)]
pub enum ServerCmd {
    LogDump {
        data: Vec<LogCommand>,
        log_type: LogType,
    },
}

#[derive(Serialize)]
pub enum OutputData {
    Engine(Command),
    Server(ServerCmd),
}

#[derive(Serialize)]
pub struct ServerOutput {
    pub time: Time,
    pub view_gate: ViewGate,
    pub data: OutputData,
}

// a batch can either be live, or it can be an initial batch which tells a client to initialize their
// state, or reset their current state and construct a new state with the initialize batch.
// widening?
// widening becomes a rescan under the actor's new permissions.
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

#[derive(Serialize)]
pub struct Batch {
    // the response pair is sent only to the connection which triggered the batch
    pub kind: BatchKind,
    // outputs are sent to everyone who passes the view gate
    pub outputs: Vec<ServerOutput>,
}
