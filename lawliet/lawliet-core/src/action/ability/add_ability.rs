/*
* SYSTEM ACTION
* Add an ability to the world
*/

// TODO:
// - Optimize this by just constructing a link set and passing it directly into the ability constructor

use crate::{
    ability::Ability,
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, AddChargePool,
    },
    chargepool::{ChargeConditions, PoolLink},
    common::AbilityKey,
    config::ability::{AbilityIdentifier, ConfigPoolLinkDetails},
    helpers::get_charge_pool_mut,
};

pub use crate::action::{AddAbility, AddAbilityResponse};

impl ActionInterface for AddAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let Some(config) = eng.config.abilities.get(&AbilityIdentifier {
            name: self.ability_name,
            variant: self.variant,
        }) else {
            return Err(ActionError::AbilityConfigNotFound);
        };

        // Only create non-volatile links on ability creation. Each entry carries the
        // config link's subtraction condition alongside its resolved PoolLink.
        let conf_links = &config.default_links.clone();
        let mut links_to_create: Vec<(PoolLink, ChargeConditions)> = vec![];
        for link in conf_links {
            match &link.details {
                ConfigPoolLinkDetails::Individual(specifier) => {
                    let response = Action::AddChargePool(AddChargePool {
                        base_charges: specifier.charges,
                        base_reset_time: specifier.reset_time,
                    })
                    .handle(eng, ctx, actor, version, mutate)?;
                    let ActionResponse::AddChargePool(data) = response else {
                        unreachable!()
                    };
                    // the pool is only really created on the mutation path
                    if mutate {
                        let pool = get_charge_pool_mut(eng, data.id)?;
                        pool.on_link();
                        links_to_create.push((
                            PoolLink {
                                link_dest: data.id,
                                weight: link.weight,
                                link_type: link.link_type,
                            },
                            link.condition,
                        ));
                    }
                }
                ConfigPoolLinkDetails::World(pool_name) => {
                    links_to_create.push((
                        PoolLink {
                            link_type: link.link_type,
                            weight: link.weight,
                            link_dest: *eng.world.pool_map.get(pool_name).unwrap(), // crash on
                                                                                    // failure. it must have been created before any abilities.
                        },
                        link.condition,
                    ));
                }
                ConfigPoolLinkDetails::Actor(_) => {} // if its an
                                                      // actor, it only binds when the ability changes owners
            }
        }

        let id = if mutate {
            let mut ability = Ability::new(self.ability_name, self.variant, self.transferrable);
            for (link, condition) in &links_to_create {
                ability.add_link(
                    link.link_dest,
                    link.link_type,
                    link.weight,
                    false,
                    *condition,
                );
            }
            for (link, _) in &links_to_create {
                let pool = get_charge_pool_mut(eng, link.link_dest)?;
                pool.on_link();
            }
            eng.world.add_ability(ability)
        } else {
            AbilityKey::default()
        };

        Ok(ActionResponse::AddAbility(AddAbilityResponse { id }))
    }
}
