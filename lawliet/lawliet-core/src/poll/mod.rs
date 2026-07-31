use indexmap::{IndexMap, IndexSet};
use smallvec::{SmallVec, smallvec};

use crate::{
    action::CreatePoll,
    common::{ActorKey, PollWeight, TimerKey, ViewportKey},
    engine::Engine,
    helpers::{get_channel, get_org, get_voter_weight},
    poll::policies::{
        resolution::{majority, most_voted},
        voter::present,
    },
};
mod policies;

// polls have resolution policies which determine if the poll resolves or not
// a poll may resolve immediately when some threshold is reached, or it may
// resolve after the poll times out
//
// polls also have valid voter policies which decide if a vote is valid (i.e., the vote counts if it
// is already in the set, or whether or not the vote is even added to the set)
//
// polls can only run while their attached action is possible. if for any reason the action's
// validation pass rejects, the poll will cancel itself.
//
// some examples:
// - org polls typically resolve immediately when majority is reached, and if majority is not met by
// the timeout, the poll is inconclusive
// - courtroom polls will only resolve after timing out. which ever side gets the most votes wins.
// if the vote counts are equal, the poll is inconclusive, and the player walks free.
//
// this behaviour is implemented as such:
// - polls have two policies: update and timeout
// - policies may return inconclusive, success, or reject
// - if an update policy returns inconclusive, nothing happens
// - if an update policy returns reject or accept, the poll concludes
// - a poll will always conclude with the return of a timeout policy
//
// polls now have individual accept and reject actions

pub use lawliet_types::poll::{
    PollOption, PollOptionIndex, PollOptionLabel, PollOptionTally, PollOutcome, PollParent,
    PollPolicy, PollSubject, VoterPolicy,
};

#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum PolicyResult {
    // The option that won.
    Resolved(PollOptionIndex),
    Inconclusive,
}

// The weight behind each option, in the poll's own option order, and the weight that could
// possibly be cast. Every resolution policy is a question about these two.
#[derive(Debug)]
pub struct VoteQuery {
    pub options: SmallVec<[PollWeight; 4]>,
    pub potential_total: PollWeight,
}

#[derive(Debug)]
pub struct Poll {
    pub options: Vec<PollOption>,
    pub parent: PollParent,
    pub subject: PollSubject,
    pub update_policy: PollPolicy,
    pub timeout_policy: PollPolicy,
    pub voter_policy: VoterPolicy,
    pub ignore_amplification: bool,
    // Who opened the poll (None = no distinct opener, e.g. a system-driven vote). Stored so
    // every broadcast can carry it; the client shows it on the "vote started" notice.
    pub opener: Option<ActorKey>,
    // Each voter's chosen option, by index into `options`.
    pub votes: IndexMap<ActorKey, PollOptionIndex>,
    // The countdown to this poll's timeout. None for a poll with no deadline, which runs until
    // its update policy resolves it or something tears it down. Set by CreatePoll once the poll
    // has a key to fire at, and freed by PollCleanup.
    pub timer: Option<TimerKey>,
}

impl Poll {
    // Built from the action rather than from a long positional list: update_policy and
    // timeout_policy are the same type and mean opposite things, and named fields are the only
    // way that swap cannot happen silently.
    pub fn new(action: &CreatePoll) -> Self {
        Poll {
            options: action.options.clone(),
            parent: action.parent,
            subject: action.subject.clone(),
            update_policy: action.update_policy,
            timeout_policy: action.timeout_policy,
            voter_policy: action.voter_policy,
            ignore_amplification: action.ignore_amplification,
            opener: action.opener,
            votes: IndexMap::new(),
            timer: None,
        }
    }

    // Everyone the poll is currently addressed to: whoever the parent's viewport holds.
    pub fn viewers(&self, eng: &Engine) -> IndexSet<ActorKey> {
        self.viewport(eng)
            .and_then(|id| eng.world.get_viewport(id))
            .map(|viewport| viewport.members().collect())
            .unwrap_or_default()
    }

    fn policy(&self, pol: PollPolicy, eng: &Engine) -> PolicyResult {
        match pol {
            PollPolicy::AlwaysInconclusive => PolicyResult::Inconclusive,
            PollPolicy::Majority => majority(self, eng),
            PollPolicy::MostVoted => most_voted(self, eng),
        }
    }

    // Whose vote COUNTS. Everything downstream of the tally reads this — the weights, the
    // potential total, and so every resolution policy.
    pub fn counts(&self, eng: &Engine, voter_id: ActorKey) -> bool {
        match self.voter_policy {
            VoterPolicy::Present => present(self, eng, voter_id),
        }
    }

    // Who may CAST or CHANGE a vote right now. Strictly narrower than counts, and deliberately a
    // different question: you cannot enter a vote you would not be counted for, but losing sight
    // of a poll does not discard the vote you already entered. A blackout is what the gap is for
    // — the ballot shuts, the ballot box keeps what is in it, and the world can still reach a
    // conclusion on the votes it already holds.
    pub fn can_enter(&self, eng: &Engine, voter_id: ActorKey) -> bool {
        self.counts(eng, voter_id) && self.can_view(eng, voter_id)
    }

    // Who this poll is FOR, as a standing fact: the parent's MEMBERSHIP. can_view asks the same
    // question about right now, off the parent's VIEWPORT, and the two differ wherever sight is
    // transient — a blackout takes a world poll off the air without changing whose poll it is.
    pub fn in_scope(&self, eng: &Engine, id: ActorKey) -> bool {
        match self.parent {
            PollParent::Org(org_id) => get_org(eng, org_id).is_ok_and(|org| org.has_member(id)),
            PollParent::Channel(channel_id) => {
                get_channel(eng, channel_id).is_ok_and(|ch| ch.get_member(id).is_some())
            }
            // The whole world is the audience. What can make this poll unreachable without
            // un-scoping anybody is can_view's business.
            PollParent::World => true,
        }
    }

    // The viewport this poll rides. A poll is seen by exactly whoever can see the thing it was put
    // to, so it is addressed to that object's viewport rather than keeping one of its own — which
    // is also how it inherits, for free, every reason that object's audience can shrink: a
    // blackout emptying world events, a permission revoked in a channel.
    //
    // An org is seen through its channel. It owns no viewport itself, and it does not need one:
    // being in the org is being in the org's channel.
    //
    // None means the parent is gone, which is the end of the poll as well — UpdatePolls cancels
    // a poll whose parent it can no longer find.
    pub fn viewport(&self, eng: &Engine) -> Option<ViewportKey> {
        let channel_id = match self.parent {
            PollParent::World => return Some(eng.world.events_viewport),
            PollParent::Channel(channel_id) => channel_id,
            PollParent::Org(org_id) => get_org(eng, org_id).ok()?.channel_id,
        };
        Some(get_channel(eng, channel_id).ok()?.viewport)
    }

    // Can this actor SEE the poll right now. Decides delivery, and with it whether they may still
    // touch their vote.
    //
    // Sight, not membership: a channel member whose View has been revoked — because the channel
    // was blacked out, or for any other reason — is still in scope and still counts, but cannot
    // reach the ballot. The same shape the world case has.
    pub fn can_view(&self, eng: &Engine, id: ActorKey) -> bool {
        self.viewport(eng)
            .and_then(|viewport| eng.world.get_viewport(viewport))
            .is_some_and(|viewport| viewport.contains(id))
    }

    pub fn is_option(&self, option: PollOptionIndex) -> bool {
        (option as usize) < self.options.len()
    }

    pub fn update_policy(&self, eng: &Engine) -> PolicyResult {
        self.policy(self.update_policy, eng)
    }

    pub fn timeout_policy(&self, eng: &Engine) -> PolicyResult {
        self.policy(self.timeout_policy, eng)
    }

    // What one voter is worth. Clamping to one is all `ignore_amplification` means: a non-voter's
    // zero is not amplification, it is not being a voter, so it survives the clamp.
    fn weight_of(&self, eng: &Engine, id: ActorKey) -> PollWeight {
        let weight = get_voter_weight(eng, id);
        if self.ignore_amplification {
            weight.min(1)
        } else {
            weight
        }
    }

    pub fn weights(&self, eng: &Engine) -> VoteQuery {
        let mut options: SmallVec<[PollWeight; 4]> = smallvec![0; self.options.len()];
        let mut potential = 0;

        let mut weights = IndexMap::new();
        for (id, _) in eng.world.actors.iter() {
            if !self.counts(eng, id) {
                continue;
            }
            let weight = self.weight_of(eng, id);
            weights.insert(id, weight);
            potential += weight;
        }

        for (id, option) in &self.votes {
            if !self.counts(eng, *id) {
                continue;
            }
            // A vote for an option that is not there cannot happen — options are fixed at
            // creation and AddVote checks the index — so it simply contributes nothing.
            if let Some(tally) = options.get_mut(*option as usize) {
                *tally += weights.get(id).unwrap();
            }
        }

        VoteQuery {
            options,
            potential_total: potential,
        }
    }

    pub fn add_vote(&mut self, id: ActorKey, option: PollOptionIndex) {
        self.votes.insert(id, option);
    }

    pub fn remove_vote(&mut self, id: ActorKey) {
        self.votes.swap_remove(&id);
    }

    pub fn contains_voter(&self, id: ActorKey) -> bool {
        self.votes.contains_key(&id)
    }
}
