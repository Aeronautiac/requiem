use serde::{Deserialize, Serialize};

use crate::{
    ability::AbilityBehaviour,
    action::Action,
    common::{ActorKey, ChannelKey, PollWeight},
};

// Which option a vote is for: an index into the poll's options. Options are fixed when the poll is
// created and never move, so an index is a stable name for a choice.
pub type PollOptionIndex = u8;

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum VoterPolicy {
    Present,
}

// What a poll is about, so the frontend can render it. The org/channel it belongs to is
// already carried by the poll's parent, so subjects never repeat it. `Generic`
// is the fallback for polls without a dedicated variant — it holds a pre-rendered string.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum PollSubject {
    // vote on the org (see the poll's parent) using one of its abilities. carries the full
    // behaviour (ability name + proposed arguments) so the frontend shows what's proposed.
    OrgAbility(AbilityBehaviour),
    // a public vote to jail a player; carries the arrest target.
    CivilianArrest(ActorKey),
    Generic(String),
}

// What one option on a ballot offers. The subject says what the poll is about; this says what
// this particular choice would do about it. `Generic` is the same fallback `PollSubject` has.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum PollOptionLabel {
    Accept,
    Reject,
    Generic(String),
}

// One choice on a ballot and what happens if it wins. A payload of None is a choice that only
// says no.
//
// Options are fixed for the poll's life. Votes name an option by its index, so a list that could
// move under the voters would quietly reassign their votes to something else — which is also why
// an option whose payload stops validating cancels the whole poll rather than being dropped from
// it. See UpdatePolls.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PollOption {
    pub label: PollOptionLabel,
    pub payload: Option<Action>,
}

impl PollOption {
    // The ordinary two-option ballot, and the convention its callers share: accept is option 0,
    // reject is option 1.
    pub fn accept_reject(accept: Option<Action>, reject: Option<Action>) -> Vec<PollOption> {
        vec![
            PollOption {
                label: PollOptionLabel::Accept,
                payload: accept,
            },
            PollOption {
                label: PollOptionLabel::Reject,
                payload: reject,
            },
        ]
    }
}

// One option as the client sees it: what it offers and how much weight is behind it. The whole
// list is re-sent on every tally refresh — the labels never change, but a client rebuilding from
// the command stream should not have to hold a poll's shape and its numbers in two places.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PollOptionTally {
    pub label: PollOptionLabel,
    pub weight: PollWeight,
}

// How a poll ended, for the resolution notice. `Resolved` names the option that won, which the
// frontend already has a label for from the poll's own data.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PollOutcome {
    Resolved(PollOptionIndex),
    Inconclusive,
    Cancelled,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum PollPolicy {
    AlwaysInconclusive,
    // An option holds more than half of the weight that could possibly be cast.
    Majority,
    // The option with the most weight behind it. A tie is inconclusive, and so is a poll nobody
    // voted in.
    MostVoted,
}

// What a poll hangs off. One parent answers two separate questions and they must not be confused
// for each other: its MEMBERSHIP is who the poll is put to (whose vote counts, a standing fact),
// and its VIEWPORT is who can reach the ballot right now (whose vote can be entered). A blackout
// moves the second without touching the first, which is the whole reason a vote cast in the light
// still counts once the lights go out.
//
// A poll has no viewport of its own — it rides its parent's, and dies when its parent does.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum PollParent {
    Org(ActorKey),
    Channel(ChannelKey),
    World,
}
