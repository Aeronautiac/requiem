/*
* PLAYER ONLY
* Remove a vote from a poll
*/

use crate::{
    action::{
        ActionInterface, ActionError, ActionResponse,
    },
    common::PollKey,
    helpers::{actor_id, get_poll, get_poll_mut},
};

use crate::action::ActionActor;
pub use crate::action::{RemoveVote, RemoveVoteResponse};

impl ActionInterface for RemoveVote {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_only()?;
        let player_id = actor_id(actor).unwrap();

        let poll = get_poll(eng, self.poll_id)?;
        if !poll.voter_policy(eng, player_id) {
            return Err(ActionError::InvalidVoter);
        }
        if !poll.contains_voter(player_id) {
            return Err(ActionError::NotAVoter);
        }
        if mutate {
            let poll = get_poll_mut(eng, self.poll_id)?;
            poll.remove_vote(player_id);
        }

        super::broadcast_poll(eng, ctx, self.poll_id, mutate);

        Ok(ActionResponse::RemoveVote(RemoveVoteResponse {}))
    }
}
