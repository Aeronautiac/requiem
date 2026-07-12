/* SYSTEM ACTION
* Transfer ownership of an ability to a specified actor and then reset links
*/

// TODO:
// Handle organization transfers. Orgs have a map of ability ids to ability metadata.
// Shouild probably be done in higher level actions

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, AddLink, ClearVolatileLinks, UpdateBugVisibilities,
    },
    chargepool::{ChargeConditions, PoolLink},
    command::Command,
    config::ability::{AbilityIdentifier, ConfigPoolLinkDetails},
    helpers::{get_ability, get_ability_mut, get_actor, get_actor_mut},
};

pub use crate::action::{GiveAbility, GiveAbilityResponse};

impl ActionInterface for GiveAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        get_actor(eng, self.actor_id)?;

        let ability = get_ability(eng, self.ability_id)?;
        let name = ability.ability_name;
        let variant = ability.variant;
        let old_owner = ability.ownership_struct.owner;
        if let Some(owner) = old_owner {
            if owner == self.actor_id {
                return Err(ActionError::ItemAlreadyOwned);
            }
            if mutate {
                let other_actor = get_actor_mut(eng, owner).unwrap(); // if
                // the ability is storing the id of an actor that doesn't exist, there is something
                // wrong with the engine.
                other_actor.remove_ability(self.ability_id);
            }
        }

        Action::ClearVolatileLinks(ClearVolatileLinks {
            ability_id: self.ability_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        let Some(config) = eng
            .config
            .abilities
            .get(&AbilityIdentifier { name, variant })
        else {
            return Err(ActionError::AbilityConfigNotFound);
        };

        let actor_data = get_actor(eng, self.actor_id)?;
        let conf_links = &config.default_links.clone();
        let mut links_to_create: Vec<(PoolLink, ChargeConditions)> = vec![];
        for link in conf_links {
            if let ConfigPoolLinkDetails::Actor(pool_name) = &link.details {
                links_to_create.push((
                    PoolLink {
                        link_type: link.link_type,
                        weight: link.weight,
                        link_dest: *actor_data.pool_map.get(pool_name).unwrap(), // crash on
                                                                                 // failure. it must have been created before any abilities.
                    },
                    link.condition,
                ));
            }
        }

        let ability = get_ability_mut(eng, self.ability_id)?;
        if mutate {
            ability
                .ownership_struct
                .set_owner(self.actor_id, self.volatile);

            for (link, condition) in &links_to_create {
                Action::AddLink(AddLink {
                    ability_id: self.ability_id,
                    pool_id: link.link_dest,
                    weight: link.weight,
                    link_type: link.link_type,
                    volatile: true,
                    condition: *condition,
                })
                .handle(eng, ctx, actor, version, mutate)?;
            }

            let actor_data = get_actor_mut(eng, self.actor_id)?;
            actor_data.add_ability(self.ability_id);
        }

        Action::UpdateBugVisibilities(UpdateBugVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        if mutate {
            let ability = get_ability(eng, self.ability_id)?;
            let (success_usages_remaining, failure_usages_remaining, iterations_to_reset) =
                ability.get_ability_view_counts(eng);
            ctx.push_cmd(
                Command::UpdateAbilityView {
                    ability_name: ability.ability_name,
                    success_usages_remaining,
                    failure_usages_remaining,
                    iterations_to_reset,
                    ability_id: self.ability_id,
                    owner_id: self.actor_id,
                },
                CommandRecipient::Actor(self.actor_id),
                eng.time,
            );
        }

        Ok(ActionResponse::GiveAbility(GiveAbilityResponse {}))
    }
}
