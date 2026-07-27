use serde::{Deserialize, Serialize};

use crate::common::AbilityKey;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ProsecutionSource {
    None,
    Ability(AbilityKey),
}

// Whether the side holding the floor has started yet. Surfaced rather than collapsed because the
// two mean different things to the player who holds it: in Grace you have not begun and your first
// message starts your slot, in Presentation the clock is already running on it.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum TrialSubphaseView {
    Grace,
    Presentation,
}

// Which side currently holds the floor during the trial phase.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum TrialPhaseView {
    Prosecutor(TrialSubphaseView),
    Defense(TrialSubphaseView),
    // Carries the flags because signalling done is only meaningful against whether the other side
    // already has -- a client that cannot see that has no feedback for the action at all.
    Debate {
        prosecutor_done: bool,
        defense_done: bool,
    },
}

// The client-facing snapshot of where a prosecution is in its lifecycle. Custody doubles as the
// "someone is being prosecuted" announcement; Voting means the verdict poll is live (the poll
// itself rides the poll protocol, not this one).
//
// The ready/done flags live inside the phase that owns them rather than beside it, so a phase
// which has no such flags cannot be described as having them.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ProsecutionPhaseView {
    Custody {
        prosecutor_ready: bool,
        defense_ready: bool,
    },
    Trial(TrialPhaseView),
    Voting,
}

impl ProsecutionPhaseView {
    // Whether two snapshots are the same PHASE, ignoring the ready/done flags within it.
    //
    // This is what the frontend diffs to decide a phase actually changed and news is warranted.
    // Comparing whole values would make every signal look like a transition; ignoring the subphase
    // would swallow grace -> presentation, which is a real change worth announcing.
    pub fn same_phase(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custody { .. }, Self::Custody { .. }) | (Self::Voting, Self::Voting) => true,
            (Self::Trial(a), Self::Trial(b)) => match (a, b) {
                (TrialPhaseView::Prosecutor(a), TrialPhaseView::Prosecutor(b))
                | (TrialPhaseView::Defense(a), TrialPhaseView::Defense(b)) => a == b,
                (TrialPhaseView::Debate { .. }, TrialPhaseView::Debate { .. }) => true,
                _ => false,
            },
            _ => false,
        }
    }
}
