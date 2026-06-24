use serde::{Deserialize, Serialize};
use specta::Type;

use crate::common::{ActorKey, ChannelKey};

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize, Type)]
pub enum VoterPolicy {
    Present,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize, Type)]
pub enum PollPolicy {
    AlwaysInconclusive,
    Majority,
    WinningVote,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize, Type)]
pub enum PollVisibility {
    Org(ActorKey),
    Channel(ChannelKey),
    AllPresent,
}
