use enumflags2::{BitFlags, bitflags};
use serde::{Deserialize, Serialize};

use crate::actor::ActorDisplay;
use crate::common::{
    ActorKey, GroupchatKey, ID, KidnappingKey, LoungeKey, NotebookKey, ProfileKey, ProsecutionKey,
};
use crate::world::WorldChannelName;

// What a channel IS, stated on MapChannel when the channel is registered.
//
// Every channel in the game is an ordinary engine channel; what differs is the thing it belongs
// to. That thing is carried here rather than in a command per kind, so a frontend has one place to
// learn a channel exists and one match to decide how to present it.
//
// The payload is whatever ties the channel to its owner, and nothing else -- presentation is the
// frontend's business. A kind carrying no owner (Personal) needs no payload at all: the channel is
// its own subject, and only its owner is ever a member.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelKind {
    // There is exactly one instance of each of these in a world.
    World(WorldChannelName),
    // contact_id is the strictly-increasing contact-channel id, used for display (e.g.
    // "lounge-<contact_id>") and to reference the contact channel from a tap-in or a contact log.
    Lounge {
        lounge_id: LoungeKey,
        contact_id: ID,
    },
    Groupchat {
        gc_id: GroupchatKey,
        contact_id: ID,
    },
    Notebook(NotebookKey),
    // The org's actor id. This is the ONLY statement of which channel backs which org; MapActor
    // says the org exists and leaves the link here.
    Org(ActorKey),
    // A channel a player made for themselves: a notepad, or a line to whoever bugged them.
    Personal,
    // The private line between a defendant and their chosen lawyer.
    Lawyer(ProsecutionKey),
    // Where the trial itself is held.
    Trial(ProsecutionKey),
    Kidnapping(KidnappingKey),
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum ChannelPerm {
    Send = 1 << 0,
    View = 1 << 1,
    LoggabilityControl = 1 << 2,
}
pub type ChannelPermSet = BitFlags<ChannelPerm>;

// One profile as a client sees it: who someone may appear as in this channel, and what appearing
// as them may do.
//
// A profile is the unit of participation, so this is what both halves of the channel protocol
// carry — the visible ones go to the room on ChannelRoster, and the recipient's own go to them on
// ProfileAccess. The two are separate because a profile may be owned before it is visible, and the
// display of an unrevealed profile is exactly the thing the room must not be told.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelProfileView {
    pub profile_id: ProfileKey,
    pub display: ActorDisplay,
    pub perms: ChannelPermSet,
}

// SYSTEM-only. Who wears one name in a channel. Pairs entry-for-entry with a ChannelProfileView in
// the roster by profile_id, and carries the single thing the roster withholds from the room: the
// actors behind the name. An empty owners list is a name currently worn by nobody.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileOwners {
    pub profile_id: ProfileKey,
    pub owners: Vec<ActorKey>,
}

// A profile's permission rule. The data half only — what each one decides, and when it may be
// applied at all, is written out in lawliet_core::channel::policies.
//
// These live here rather than beside their impls because an action carries one: AddProfile names
// the rule a profile is built with, and CreateChannel names the rule its base profile is stamped
// with, so the rule has to cross the wire.

// Gate permissions on the current profile owner's contact modifier status, and provide the typical
// set of contact channel permissions.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct ContactPolicy {}

// Always return some set of permissions.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct FixedPolicy {
    pub perms: ChannelPermSet,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct NewsPolicy {}

// Grant these permissions while the owner is present, and nothing at all otherwise. All or
// nothing: a channel that needs some permissions gated one way and others another wants a policy
// of its own rather than a knob on this one.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct PresencePolicy {
    pub perms: ChannelPermSet,
}

// Talk and listen, unless the owner is dead.
//
// For channels you are in because of something that happened to you, or something you are holding:
// the prison, a kidnapping, a notebook. Being put there is what admits you, so there is no standing
// left to check — except that the dead do not speak. Notably it ignores the contact modifiers,
// since being held is what cuts your contact everywhere else and this is the place that does not
// reach.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct AlivePolicy {}

// A seat at a trial: View while present, and Send while this NAME is the one holding the floor.
//
// Every profile in a trial channel carries it, the spectators' included, and that is what keeps
// the roster uniform. It decides from the profile's display rather than from its owner, which is
// the whole trick: a trial knows its prosecutor by the display it announced them under, so a
// prosecutor named openly gets the floor on the ordinary seat everyone has, and one who is
// anonymous gets it on their mask while their own name stays as quiet as anybody else's.
//
// One policy rather than a spectator rule and a participant rule, because a participant would
// otherwise need their spectator seat edited around them — and two rules that both decide the same
// profile's permissions is an ordering problem waiting to happen.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct TrialPolicy {
    pub prosecution_id: ProsecutionKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum PermUpdatePolicy {
    Fixed(FixedPolicy),
    Contact(ContactPolicy),
    News(NewsPolicy),
    Presence(PresencePolicy),
    Alive(AlivePolicy),
    Trial(TrialPolicy),
}

// Which display a base profile is built with. A blueprint is stamped out for players who did not
// exist when the channel was made, so it names a rule for producing a display rather than a
// display.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum BlueprintDisplayKind {
    // A raw display of the owner.
    OwnerRaw,
}

// The profile every player is handed a copy of, and the thing that makes them a member at all.
//
// This is the answer to "who is in this channel" wherever the answer is everyone. A channel
// without one has its membership decided by whatever action owns it, which is the other half of
// the game — lounges, group chats, orgs, notebooks, and the world channels you are put into by
// something happening to you.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct ProfileBlueprint {
    pub start_visible: bool,
    pub perm_policy: PermUpdatePolicy,
    pub display_kind: BlueprintDisplayKind,
}
