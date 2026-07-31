use crate::{common::ActorKey, engine::Engine, helpers::get_actor, poll::Poll};

// they must not have the present restriction
// they must be inside the poll's audience
//
// Scope, not sight: whether the poll is reachable right now is can_view's question, and a voter
// whose vote has stopped being enterable has not stopped counting.
pub fn present(poll: &Poll, eng: &Engine, voter_id: ActorKey) -> bool {
    let actor = get_actor(eng, voter_id).unwrap(); // the actor id must be valid,
    // if it isnt, the engine is broken
    if actor.has_modifier(crate::actor::modifier::Modifier::NoPresence) {
        return false;
    }
    poll.in_scope(eng, voter_id)
}
