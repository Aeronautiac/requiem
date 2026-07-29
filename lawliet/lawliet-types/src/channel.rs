use enumflags2::{BitFlags, bitflags};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::actor::ActorDisplay;
use crate::common::{ActorKey, GroupchatKey, ID, LoungeKey, NotebookKey, ProsecutionKey};
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
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum ChannelPermission {
    Send = 1 << 0,
    View = 1 << 1,
    LoggabilityControl = 1 << 2,
}
pub type ChannelPermissions = BitFlags<ChannelPermission>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMember {
    pub perms: ChannelPermissions,
    pub displays: IndexSet<ActorDisplay>,
}
