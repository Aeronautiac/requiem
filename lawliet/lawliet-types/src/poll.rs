use serde::{Deserialize, Serialize};

use crate::{
    ability::AbilityBehaviour,
    common::{ActorKey, ChannelKey},
};

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum VoterPolicy {
    Present,
}

// What a poll is about, so the frontend can render it. The org/channel it belongs to is
// already carried by the poll's scope (visibility), so subjects never repeat it. `Generic`
// is the fallback for polls without a dedicated variant — it holds a pre-rendered string.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum PollSubject {
    // vote on the org (see the poll's scope) using one of its abilities. carries the full
    // behaviour (ability name + proposed arguments) so the frontend shows what's proposed.
    OrgAbility(AbilityBehaviour),
    // a public vote to jail a player; carries the arrest target.
    CivilianArrest(ActorKey),
    Generic(String),
}

// How a poll ended, for the resolution notice.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PollOutcome {
    Accepted,
    Rejected,
    Inconclusive,
    Cancelled,
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
