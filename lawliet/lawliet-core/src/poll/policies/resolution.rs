use crate::{
    engine::Engine,
    poll::{PolicyResult, Poll},
};

// if the percentage of one side is > 50% of the total vote weight, that side wins
// otherwise, inconclusive
pub fn majority(poll: &Poll, eng: &Engine) -> PolicyResult {
    let query = poll.weights(eng);
    if 2 * query.accept > query.potential_total {
        return PolicyResult::Accept;
    }
    if 2 * query.reject > query.potential_total {
        return PolicyResult::Reject;
    }
    PolicyResult::Inconclusive
}

// if the weight of one side > the other, that side wins
// if the weights are equal, inconclusive pub fn winning_vote(poll: &Poll, eng: &Engine) -> PolicyResult {
pub fn winning_vote(poll: &Poll, eng: &Engine) -> PolicyResult {
    let query = poll.weights(eng);
    dbg!(&query);
    if query.accept > query.reject {
        PolicyResult::Accept
    } else if query.reject > query.accept {
        PolicyResult::Reject
    } else {
        PolicyResult::Inconclusive
    }
}
