/*
* SYSTEM ACTION
* Remove a state and its associated restrictions from an actor
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        UpdateBugVisibilities, UpdateContactChannels, UpdateKidnapChannels, UpdatePrisonChannel,
        UpdateWorldChannelPerms,
    },
    common::Version,
    engine::Engine,
    helpers::{get_actor_mut, get_player},
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
        if mutate {
            target.remove_state(self.state);
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

        Action::UpdateBugVisibilities(UpdateBugVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdateKidnapChannels(UpdateKidnapChannels {})
            .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdatePrisonChannel(UpdatePrisonChannel {
            actor_id: self.actor_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::RemoveState(RemoveStateResponse {}))
    }
}
