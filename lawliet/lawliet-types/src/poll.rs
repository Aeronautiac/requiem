use serde::{Deserialize, Serialize};

use crate::common::{ActorKey, ChannelKey};

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum VoterPolicy {
    Present,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum PollPolicy {
    AlwaysInconclusive,
    Majority,
    WinningVote,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum PollVisibility {
    Org(ActorKey),
    Channel(ChannelKey),
    AllPresent,
}
