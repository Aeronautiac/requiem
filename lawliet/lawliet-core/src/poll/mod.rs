use indexmap::IndexMap;

use crate::{
    action::Action,
    common::{ActorKey, ChannelKey, PollWeight},
    engine::Engine,
    helpers::get_voter_weight,
    poll::policies::{
        resolution::{majority, winning_vote},
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

pub use lawliet_types::poll::{VoterPolicy, PollPolicy, PollVisibility};

#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum PolicyResult {
    Accept,
    Reject,
    Inconclusive,
}

#[derive(Debug)]
pub struct Vote {
    pub accept: bool,
}

#[derive(Debug)]
pub struct VoteQuery {
    pub accept: PollWeight,
    pub reject: PollWeight,
    pub total: PollWeight,
    pub potential_total: PollWeight,
}

#[derive(Debug)]
pub struct Poll {
    pub accept_payload: Option<Action>,
    pub reject_payload: Option<Action>,
    pub visibility: PollVisibility,
    pub update_policy: PollPolicy,
    pub timeout_policy: PollPolicy,
    pub voter_policy: VoterPolicy,
    pub votes: IndexMap<ActorKey, Vote>,
}

impl Poll {
    pub fn new(
        accept_payload: Option<Action>,
        reject_payload: Option<Action>,
        visibility: PollVisibility,
        update_policy: PollPolicy,
        timeout_policy: PollPolicy,
        voter_policy: VoterPolicy,
    ) -> Self {
        Poll {
            accept_payload,
            reject_payload,
            visibility,
            update_policy,
            timeout_policy,
            voter_policy,
            votes: IndexMap::new(),
        }
    }

    fn policy(&self, pol: PollPolicy, eng: &Engine) -> PolicyResult {
        match pol {
            PollPolicy::AlwaysInconclusive => PolicyResult::Inconclusive,
            PollPolicy::Majority => majority(self, eng),
            PollPolicy::WinningVote => winning_vote(self, eng),
        }
    }

    pub fn voter_policy(&self, eng: &Engine, voter_id: ActorKey) -> bool {
        match self.voter_policy {
            VoterPolicy::Present => present(self, eng, voter_id),
        }
    }

    pub fn update_policy(&self, eng: &Engine) -> PolicyResult {
        self.policy(self.update_policy, eng)
    }

    pub fn timeout_policy(&self, eng: &Engine) -> PolicyResult {
        self.policy(self.timeout_policy, eng)
    }

    pub fn weights(&self, eng: &Engine) -> VoteQuery {
        let mut accept = 0;
        let mut reject = 0;
        let mut potential = 0;

        let mut weights = IndexMap::new();
        for (id, _) in eng.world.actors.iter() {
            if !self.voter_policy(eng, id) {
                continue;
            }
            let weight = get_voter_weight(eng, id);
            weights.insert(id, weight);
            potential += weight;
        }

        for (id, vote) in &self.votes {
            if !self.voter_policy(eng, *id) {
                continue;
            }
            let weight = weights.get(id).unwrap();
            if vote.accept {
                accept += weight;
            } else {
                reject += weight;
            }
        }

        VoteQuery {
            accept,
            reject,
            total: accept + reject,
            potential_total: potential,
        }
    }

    pub fn add_vote(&mut self, id: ActorKey, accept: bool) {
        self.votes.insert(id, Vote { accept });
    }

    pub fn remove_vote(&mut self, id: ActorKey) {
        self.votes.swap_remove(&id);
    }

    pub fn contains_voter(&self, id: ActorKey) -> bool {
        self.votes.contains_key(&id)
    }
}
