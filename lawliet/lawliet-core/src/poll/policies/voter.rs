use crate::{
    common::ActorKey,
    engine::Engine,
    helpers::{get_actor, get_channel, get_org},
    poll::{Poll, PollVisibility},
};

fn visibility_check(poll: &Poll, eng: &Engine, voter_id: ActorKey) -> bool {
    match poll.visibility {
        PollVisibility::Org(org_id) => {
            let org = get_org(eng, org_id).unwrap();
            org.has_member(voter_id)
        }
        PollVisibility::Channel(channel_id) => {
            let channel = get_channel(eng, channel_id).expect(
                "invariant violated: expected valid channel id within a poll visibility enum",
            );
            channel.get_member(voter_id).is_some()
        }
        PollVisibility::AllPresent => true,
    }
}

// they must not have the present restriction
// they must be able to see the vote
pub fn present(poll: &Poll, eng: &Engine, voter_id: ActorKey) -> bool {
    let actor = get_actor(eng, voter_id).unwrap(); // the actor id must be valid,
    // if it isnt, the engine is broken
    if actor.has_modifier(crate::actor::modifier::Modifier::NoPresence) {
        return false;
    }
    visibility_check(poll, eng, voter_id)
}
