use serde::{Deserialize, Serialize};

use crate::common::VoteAmplifier;

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum ContactLogType {
    Full,
    Even,
    Odd,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PassiveType {
    Wanted,
    VoteAmplication { multiplier: VoteAmplifier },
    VolatileEyes,
    ContactLogs(ContactLogType),
    OwnedNotebookBlock,
    CustodyBugReceiver,
}
