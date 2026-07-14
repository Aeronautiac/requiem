/*
* PLAYER ONLY
* Add a vote to a poll
*/

use crate::{
    action::{
        ActionInterface, ActionError, ActionResponse,
    },
    helpers::{actor_id, get_poll, get_poll_mut},
};

use crate::action::ActionActor;
pub use crate::action::{AddVote, AddVoteResponse};

impl ActionInterface for AddVote {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_only()?;
        let player_id = actor_id(actor).unwrap();

        let poll = get_poll(eng, self.poll_id)?;
        if !poll.voter_policy(eng, player_id) {
            return Err(ActionError::InvalidVoter);
        }
        if poll.contains_voter(player_id) {
            return Err(ActionError::AlreadyVoted);
        }
        if mutate {
            let poll = get_poll_mut(eng, self.poll_id)?;
            poll.add_vote(player_id, self.accept);
        }

        super::broadcast_poll(eng, ctx, self.poll_id, mutate);

        Ok(ActionResponse::AddVote(AddVoteResponse {}))
    }
}
