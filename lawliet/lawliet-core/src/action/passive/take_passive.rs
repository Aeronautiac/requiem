/*
* SYSTEM ACTION
* Take a passive off its owner, leaving it in the world unowned. The inverse of GivePassive: it
* clears the owner cache and hides the passive from the former owner. The passive itself survives,
* ready to be handed to someone else.
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, UpdateContactLogViewports,
    },
    command::Command,
    helpers::{get_actor_mut, get_passive, get_passive_mut, owner_view_recipient},
};

pub use crate::action::{TakePassive, TakePassiveResponse};

impl ActionInterface for TakePassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let passive = get_passive(eng, self.passive_id)?;
        let Some(old_owner) = passive.ownership_struct.owner else {
            return Err(ActionError::ItemAlreadyUnowned);
        };

        if mutate {
            get_passive_mut(eng, self.passive_id)?.ownership_struct.owner = None;
            get_actor_mut(eng, old_owner)
                .expect("passive owner missing: engine invariant violated")
                .remove_passive(self.passive_id);
            ctx.push_cmd(
                Command::RemovePassive {
                    passive_id: self.passive_id,
                },
                owner_view_recipient(eng, old_owner),
                eng.time,
            );
        }

        // Ownership moved (to nobody), so who effectively reads this passive's contact log moved
        // with it.
        Action::UpdateContactLogViewports(UpdateContactLogViewports {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::TakePassive(TakePassiveResponse {}))
    }
}
