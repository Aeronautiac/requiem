/*
* SYSTEM ACTION
* Add states and any associated restrictions found in engine config to an actor
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        UpdateBugVisibilities, UpdateContactChannels, UpdateKidnapChannels,
        UpdatePassiveVisibilities, UpdatePrisonChannel,
        UpdateWorldChannelPerms,
    },
    common::Version,
    engine::Engine,
    helpers::{cmd_actor_state, get_actor_mut, get_player, sync_presence},
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

        if get_player(eng, self.actor_id).is_ok() {
            Action::UpdateContactChannels(UpdateContactChannels {
                player_id: self.actor_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;

            Action::UpdateWorldChannelPerms(UpdateWorldChannelPerms {
                player_id: self.actor_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        // Presence is derived from modifiers, and modifiers are only ever written here and in
        // RemoveState, so these two actions are the whole of where world-event visibility can
        // change. Resync before anything downstream emits: a player who has just lost presence
        // must already be out of the viewport when the caller announces what happened to them —
        // Kill adds State::Dead and then announces the death.
        sync_presence(eng, ctx, mutate);

        Action::UpdateBugVisibilities(UpdateBugVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        // DisablePassiveLinks rides on a state, so who reaches which passive log can change here.
        Action::UpdatePassiveVisibilities(UpdatePassiveVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdateKidnapChannels(UpdateKidnapChannels {})
            .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdatePrisonChannel(UpdatePrisonChannel {
            actor_id: self.actor_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::AddState(AddStateResponse {}))
    }
}
