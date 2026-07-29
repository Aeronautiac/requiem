// Everything that crosses the socket, in one place.
//
// amane's bindings.ts is hand-written and must mirror these exactly (no codegen, deliberately), so
// keeping them together is what makes the two sides diffable by eye. Nothing here should carry
// server internals -- if a type needs a Key or a GameHandle to be understood, it belongs elsewhere.

use lawliet_types::{
    action::{ActionError, ActionRequest, ActionResponse},
    command::CommandPayload,
    common::ActorKey,
};
use serde::{Deserialize, Serialize};

use crate::auth::{ActorScope, Capability, Key};

// what a control produced. the game-administration counterpart to ActionResponse.
#[derive(Serialize)]
pub enum ControlResponse {
    Ended,
    KeyCreated { key: Key },
    KeyRevoked,
    CapabilitiesSet,
    ActorScopeSet,
    ProfileSet,
}

// a control refused on its own terms -- the caller IS an administrator, but not for this particular
// target. distinct from Denied, which means they are not an administrator at all.
#[derive(Serialize)]
pub enum ControlError {
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
pub enum ExecOutcome {
    Action(ActionOutcome),
    Control(ControlOutcome),
}

// the ActionContext is NOT inlined here -- its commands ride the enclosing Batch's command buffer, so
// a reply and a push deliver state by the exact same path and the client has one place to apply from.
#[derive(Serialize)]
pub enum ActionOutcome {
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
pub enum ControlOutcome {
    Ok(ControlResponse),
    Err(ControlError),
    // same meaning as its ActionOutcome twin: this key does not permit what was asked. here it means
    // the key holds no administration capability at all.
    Denied,
}

// the reply echoes the input it answers, so it covers actions and controls alike -- and the client
// can match a reply to what it sent without the server inventing a correlation id.
#[derive(Serialize)]
pub struct ResponsePair {
    pub input: ServerInput,
    pub output: ExecOutcome,
}

// commands are already recipient-filtered for the connection this is addressed to. `response` is set
// only on the connection that submitted the action.
#[derive(Serialize)]
pub struct Batch {
    pub commands: Vec<CommandPayload>,
    pub response: Option<ResponsePair>,
}

// What the SERVER knows about whoever occupies an actor slot.
//
// The engine emits MapActor to say a slot exists, and that is the whole of what it knows. Who is
// playing it is a different fact with a different lifetime: it can be set after the slot exists,
// changed later, and it survives nothing the engine would call an event. So it rides its own
// channel rather than a synthetic Command, which would make lawliet declare a variant its engine
// never emits.
//
// Deliberately a profile rather than a name. Presentation is going to grow (avatars, account
// identity, whether anyone is currently connected); each of those is a field here and a control of
// its own, not another parallel channel.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Profile {
    // None = the slot exists but nobody has named it yet.
    pub display_name: Option<String>,
}

// Profiles REPLACE, per actor, exactly as the key mutators in GameControl do: an entry states the
// complete profile for that actor, so there is no read-modify-write to get wrong. Actors not
// mentioned are untouched.
//
// A profile may only ever be sent for an actor whose MapActor this connection has ALREADY been
// delivered. Otherwise this channel becomes a second, ungated way to learn that a player exists,
// and it would announce people to viewers the command stream deliberately kept them from. See
// ViewportCursor::known_actors, which is the record of what a connection has been told.
//
// Carried by the enclosing seq_num like everything else, so it cannot race the commands it
// describes.
#[derive(Serialize)]
pub struct ProfileUpdate {
    pub profiles: Vec<(ActorKey, Profile)>,
}

// Boxing the big variant would trade a heap allocation on the COMMON path (every batch) to shrink a
// value that is serialized and dropped immediately. The size only ever costs us one outbox slot per
// queued output, which is bounded by OUTBOX_BUF_SIZE.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize)]
pub enum OutputData {
    Batch(Batch),
    Profiles(ProfileUpdate),
}

#[derive(Serialize)]
pub struct ServerOutput {
    pub seq_num: u64,
    pub data: OutputData,
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
pub enum GameControl {
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
    // state what the server knows about whoever is playing an actor slot. Separate from creating
    // the slot, deliberately: the engine's AddPlayer knows nothing about presentation, a slot can
    // exist before anyone is on it, and a profile can change afterwards without the engine ever
    // hearing about it.
    //
    // Replaces, like the two above -- one control for the whole profile rather than one per field,
    // because every part of a profile has identical semantics (server-level, replaced wholesale,
    // and gated on the same MapActor). Adding a field to Profile must not add a control.
    SetProfile {
        actor: ActorKey,
        profile: Profile,
    },
}

#[derive(Serialize, Deserialize)]
pub enum ServerInput {
    Action(ActionRequest),
    Control(GameControl),
}
