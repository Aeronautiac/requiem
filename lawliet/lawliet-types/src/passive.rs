use serde::{Deserialize, Serialize};
use specta::Type;

use crate::common::VoteAmplifier;

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize, Type)]
pub enum ContactLogType {
    Full,
    Even,
    Odd,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize, Type)]
pub enum PassiveType {
    Wanted,
    VoteAmplication { multiplier: VoteAmplifier },
    VolatileEyes,
    ContactLogs(ContactLogType),
    OwnedNotebookBlock,
    CustodyBugReceiver,
}
