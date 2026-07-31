use crate::{
    engine::Engine,
    poll::{PolicyResult, Poll, PollOptionIndex},
};

// an option holding more than half of the weight that could possibly be cast wins
// otherwise, inconclusive
//
// Only one option can ever clear that bar, so the first match is the only match.
pub fn majority(poll: &Poll, eng: &Engine) -> PolicyResult {
    let query = poll.weights(eng);
    for (index, weight) in query.options.iter().enumerate() {
        if 2 * weight > query.potential_total {
            return PolicyResult::Resolved(index as PollOptionIndex);
        }
    }
    PolicyResult::Inconclusive
}

// the option with the most weight behind it wins
// if two options are level, inconclusive — and so is a poll nobody has voted in, which would
// otherwise hand a single-option ballot to the option nobody chose
pub fn most_voted(poll: &Poll, eng: &Engine) -> PolicyResult {
    let query = poll.weights(eng);
    let Some(most) = query.options.iter().copied().max().filter(|most| *most > 0) else {
        return PolicyResult::Inconclusive;
    };
    let mut winners = query
        .options
        .iter()
        .enumerate()
        .filter(|(_, w)| **w == most);
    let (index, _) = winners.next().expect("the max is one of the options");
    if winners.next().is_some() {
        return PolicyResult::Inconclusive;
    }
    PolicyResult::Resolved(index as PollOptionIndex)
}
