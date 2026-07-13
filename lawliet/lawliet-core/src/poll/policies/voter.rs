use crate::{
    common::ActorKey,
    engine::Engine,
    helpers::get_actor,
    poll::Poll,
};

// they must not have the present restriction
// they must be able to see the vote
pub fn present(poll: &Poll, eng: &Engine, voter_id: ActorKey) -> bool {
    let actor = get_actor(eng, voter_id).unwrap(); // the actor id must be valid,
    // if it isnt, the engine is broken
    if actor.has_modifier(crate::actor::modifier::Modifier::NoPresence) {
        return false;
    }
    poll.can_view(eng, voter_id)
}
