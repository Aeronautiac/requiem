use serde::{Deserialize, Serialize};

use crate::{
    actor::ActorDisplay,
    common::{ID, VoteAmplifier},
};

// Which slice of the contact graph a log passive receives, split on the parity of the contact
// channel's id. A contact belongs to one half for its whole life, so Even and Odd each see complete
// relationships rather than fragments of every one.
#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum ContactLogType {
    Full,
    Even,
    Odd,
}

impl ContactLogType {
    pub fn covers(&self, contact_id: ID) -> bool {
        match self {
            Self::Full => true,
            Self::Even => contact_id.is_multiple_of(2),
            Self::Odd => !contact_id.is_multiple_of(2),
        }
    }
}

// What happened to the contact graph, naming which KIND of channel it was: a private line and a
// groupchat seat mean different things, and the contact id alone does not say which you are looking
// at.
//
// A lounge has no closing counterpart. It is a one-to-one line, and the fact that it was opened is
// the fact worth having — leaving it later does not unmake the contact. A groupchat is an ongoing
// roster, so both edges of its membership matter.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum ContactEvent {
    LoungeOpened,
    GroupchatAdded,
    GroupchatRemoved,
}

// One line in a contact log: who reached whom, over which contact channel. No message content — a
// contact log is the shape of the graph, not what was said across it. The time is carried by the
// command payload.
//
// Both ends are DISPLAYS, not keys, because a log records what the contact looked like. An
// anonymous lounge reads as "some Watari contacted <player>", and a fabricated one reads as a
// contact that never happened — a contact log is precisely the thing those are built to fool, so it
// must be able to carry the lie.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub struct ContactLog {
    pub contact_id: ID,
    pub contactor: ActorDisplay,
    pub contacted: ActorDisplay,
    pub event: ContactEvent,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PassiveType {
    Wanted,
    VoteAmplification { multiplier: VoteAmplifier },
    VolatileEyes,
    ContactLogs(ContactLogType),
    OwnedNotebookBlock,
    CustodyBugReceiver,
}
