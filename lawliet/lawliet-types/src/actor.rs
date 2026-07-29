use enumflags2::{BitFlags, bitflags};
use serde::{Deserialize, Serialize};

use crate::common::{ActorKey, ID};
use crate::organization::OrganizationName;
use crate::role::Role;

// What kind of actor a slot holds, stated on MapActor when the slot is registered.
//
// A player carries nothing: the raw slot existing is the whole of what the engine has to say about
// it. No presentation rides here — a display name is a server-level fact about WHO is playing the
// slot, with a different lifetime, and it arrives on its own channel. (`true_name` is deliberately
// not it either: that is a MECHANIC, secret, and the thing written in a notebook.)
//
// An org carries its name, because unlike a player's that IS engine state. Which channel backs it
// is not here — MapChannel states that, on the channel.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorKind {
    Player,
    Org(OrganizationName),
}

#[derive(Hash, PartialEq, Eq, Debug, Ord, PartialOrd, Clone, Copy, Serialize, Deserialize)]
pub enum ActorLinkType {
    Life,
    Passive,
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum ActorDisplay {
    Raw(ActorKey),
    Org(ActorKey),
    Role(Role),
    Mysterious,
    System,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum State {
    Dead = 1 << 0,
    Incarcerated = 1 << 1,
    Ipp = 1 << 2,
    Kidnapped = 1 << 3,
    Custody = 1 << 4,
    UnderTheRadar = 1 << 5,
}

pub type States = BitFlags<State>;

#[bitflags]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum Modifier {
    NoPresence = 1 << 0,
    NoContact = 1 << 1,
    NoNotebookReceive = 1 << 2,
    NoNotebookUsage = 1 << 3,
    NoNotebookPassage = 1 << 4,
    DisablePassiveLinks = 1 << 5,
    WriteImmunity = 1 << 6,        // your name cannot be written in a notebook
    StrengthenedPresence = 1 << 7, // cannot be kidnapped and similar
    LogNullification = 1 << 8,     // messages will no longer be logged
    AbsoluteNoContact = 1 << 9,    // no contact with anybody, even in places like prison
}
pub type Modifiers = BitFlags<Modifier>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Source {
    State(State),
    Manual(ID),
}
