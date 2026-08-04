/*
* SYSTEM ACTION
* Take an ability off its owner, leaving it in the world unowned. The inverse of GiveAbility: it
* drops the owner-bound (volatile) pool links, clears the owner cache, and hides the ability from
* the former owner. The ability itself survives, ready to be handed to someone else.
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, ClearVolatileLinks, UpdateBugVisibilities,
    },
    command::Command,
    helpers::{get_ability, get_ability_mut, get_actor_mut, owner_view_recipient},
};

pub use crate::action::{TakeAbility, TakeAbilityResponse};

impl ActionInterface for TakeAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let ability = get_ability(eng, self.ability_id)?;
        let Some(old_owner) = ability.ownership_struct.owner else {
            return Err(ActionError::ItemAlreadyUnowned);
        };

        // The owner-bound pool links point at the owner's pools, so they go with the owner.
        Action::ClearVolatileLinks(ClearVolatileLinks {
            ability_id: self.ability_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        if mutate {
            get_ability_mut(eng, self.ability_id)?.ownership_struct.owner = None;
            get_actor_mut(eng, old_owner)
                .expect("ability owner missing: engine invariant violated")
                .remove_ability(self.ability_id);
            ctx.push_cmd(
                Command::RemoveAbility {
                    ability_id: self.ability_id,
                },
                owner_view_recipient(eng, old_owner),
                eng.time,
            );
        }

        // Ownership moved (to nobody), so any bug sourced from this ability is re-evaluated.
        Action::UpdateBugVisibilities(UpdateBugVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::TakeAbility(TakeAbilityResponse {}))
    }
}
