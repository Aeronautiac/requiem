use serde::{Deserialize, Serialize};

use crate::common::AbilityKey;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ProsecutionSource {
    None,
    Ability(AbilityKey),
}

// Which side currently holds the floor during the trial phase. Grace/presentation subphases
// aren't distinguished here — the frontend only needs to know whose turn it is.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum TrialPhaseView {
    Prosecutor,
    Defense,
    Debate,
}

// The client-facing snapshot of where a prosecution is in its lifecycle. Custody doubles as the
// "someone is being prosecuted" announcement; Voting means the verdict poll is live (the poll
// itself rides the poll protocol, not this one).
#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ProsecutionPhaseView {
    Custody,
    Trial(TrialPhaseView),
    Voting,
}
