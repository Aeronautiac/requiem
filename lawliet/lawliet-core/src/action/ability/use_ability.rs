/*
* PLAYER & ORG ONLY
* Use an ability
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    ability::{AbilityInterface, AbilityStatus},
    action::{
        ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse, ActionResult,
    },
    actor::modifier::Modifier,
    chargepool::{ChargeCondition, PoolLinkType},
    command::Command,
    helpers::{
        actor_id, get_ability, get_ability_config, get_ability_mut, get_actor, get_charge_pool_mut,
    },
};

pub use crate::action::{UseAbility, UseAbilityResponse};

impl ActionInterface for UseAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.require_not_system()?;
        let actor_id = actor_id(actor).unwrap();
        let config = get_ability_config(eng, self.ability_id)?;
        let req_presence = config.require_presence;

        let actor_data = get_actor(eng, actor_id)?;
        if req_presence && actor_data.has_modifier(Modifier::NoPresence) {
            return Err(ActionError::AbilityCategoryBlocked);
        }

        let ability = get_ability(eng, self.ability_id)?;
        if Some(actor_id) != ability.ownership_struct.owner {
            return Err(ActionError::AbilityNotOwned);
        }
        if ability.ability_name != self.ability_args.ability_name() {
            return Err(ActionError::AbilityMismatch);
        }

        // Gate usage on the current charges WITHOUT subtracting yet: every restrictive
        // link must be usable, and at least one permissive link (if any) must be usable.
        // Subtraction is deferred until after the ability body reports its status, so it
        // can be made conditional per link.
        let ability = get_ability_mut(eng, self.ability_id)?;
        let links = ability.pool_links.clone();
        let mut pool_condition = None;
        for link in &links {
            let pool = get_charge_pool_mut(eng, link.link.link_dest)?;
            match link.link.link_type {
                PoolLinkType::Restrictive => {
                    if !pool.can_use(&link.link) {
                        return Err(ActionError::AbilityNotEnoughCharges);
                    }
                }
                PoolLinkType::Permissive => {
                    if pool_condition.is_none() {
                        pool_condition = Some(false);
                    }
                    if pool.can_use(&link.link) {
                        pool_condition = Some(true);
                    }
                }
            }
        }
        if Some(false) == pool_condition {
            return Err(ActionError::AbilityNotEnoughCharges);
        }

        let status = self
            .ability_args
            .handle(eng, ctx, actor, self.ability_id, version, mutate)?;

        // Now subtract each linked pool, but only where its condition matches the
        // reported status (e.g. a link marked OnSuccess is untouched on a Failure). A
        // pool is only actually decremented when it still has the charges — matching the
        // old behavior for permissive links that may not individually be usable.
        if mutate {
            let status_flag = match status {
                AbilityStatus::Success => ChargeCondition::OnSuccess,
                AbilityStatus::Failure => ChargeCondition::OnFailure,
            };
            for link in &links {
                if link.condition.contains(status_flag) {
                    let pool = get_charge_pool_mut(eng, link.link.link_dest)?;
                    if pool.can_use(&link.link) {
                        pool.on_use(&link.link);
                    }
                }
            }
        }

        if mutate {
            let ability = get_ability(eng, self.ability_id)?;
            let ability_name = ability.ability_name;
            let (success_usages_remaining, failure_usages_remaining, iterations_to_reset) =
                ability.get_ability_view_counts(eng);
            ctx.push_cmd(
                Command::UpdateAbilityView {
                    ability_name,
                    success_usages_remaining,
                    failure_usages_remaining,
                    iterations_to_reset,
                    ability_id: self.ability_id,
                    owner_id: actor_id,
                },
                CommandRecipient::Actor(actor_id),
                eng.time,
            );
        }

        Ok(ActionResponse::UseAbility(UseAbilityResponse {}))
    }
}
