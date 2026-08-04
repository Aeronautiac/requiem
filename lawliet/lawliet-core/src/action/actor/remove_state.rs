/*
* SYSTEM ACTION
* Remove a state and its associated restrictions from an actor
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        UpdateBugVisibilities, UpdateContactLogViewports, UpdateWorldViewports,
    },
    common::Version,
    engine::Engine,
    helpers::{cmd_actor_state, get_actor_mut},
};

pub use crate::action::{RemoveState, RemoveStateResponse};

impl ActionInterface for RemoveState {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let target = get_actor_mut(eng, self.actor_id)?;
        // See AddState: removing a state the actor never had changes nothing.
        let changed = target.states.contains(self.state);
        if mutate {
            target.remove_state(self.state);
        }

        if changed {
            cmd_actor_state(eng, ctx, self.actor_id);
        }

        // See AddState. Regaining presence enters the world-events viewport, and entry backfills
        // every event that happened while the player was gone, in order.
        Action::UpdateWorldViewports(UpdateWorldViewports {})
            .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdateBugVisibilities(UpdateBugVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        // See AddState: DisablePassiveLinks coming off restores reach to a linked passive's log.
        Action::UpdateContactLogViewports(UpdateContactLogViewports {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::RemoveState(RemoveStateResponse {}))
    }
}
