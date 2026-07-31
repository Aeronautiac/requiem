/*
* SYSTEM ACTION
* Add states and any associated restrictions found in engine config to an actor
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        UpdateBugVisibilities, UpdatePassiveVisibilities, UpdateWorldViewports,
    },
    common::Version,
    engine::Engine,
    helpers::{cmd_actor_state, get_actor_mut},
};

pub use crate::action::{AddState, AddStateResponse};

impl ActionInterface for AddState {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let restrictions = eng
            .config
            .state_modifiers
            .get(&self.state)
            .cloned()
            .unwrap_or_default();

        let target = get_actor_mut(eng, self.actor_id)?;
        // Read before the write, since afterwards there is nothing to compare against. Adding a
        // state an actor already has changes nothing, and re-announcing it would be noise.
        let changed = !target.states.contains(self.state);
        if mutate {
            target.add_state(self.state, restrictions);
        }

        // The primary fact of this action; everything below is derived from it. Ahead of
        // sync_presence so it reads in order — you are told you are dead, then you leave.
        if changed {
            cmd_actor_state(eng, ctx, self.actor_id);
        }

        // Update runs this too, but not until the whole top-level action is over, and that is too
        // late: a player who has just lost presence must already be out of the world-events
        // viewport when the caller announces what happened to them — Kill adds State::Dead and
        // then announces the death. Recomputing here is idempotent and silent, so the pass in
        // Update simply finds nothing left to say.
        Action::UpdateWorldViewports(UpdateWorldViewports {})
            .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdateBugVisibilities(UpdateBugVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        // DisablePassiveLinks rides on a state, so who reaches which passive log can change here.
        Action::UpdatePassiveVisibilities(UpdatePassiveVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::AddState(AddStateResponse {}))
    }
}
