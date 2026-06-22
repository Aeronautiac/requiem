/*
* SYSTEM ACTION
* Fully destroy an ability: clear all pool links, remove from the owning actor's cache,
* drop any bugs referencing this ability, then remove from the world.
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, RemoveLink, DestroyBug, CullIncarcerations, CullKidnappings,
    },
    bug::BugSource,
    common::{AbilityKey, BugKey, ChargePoolKey},
    helpers::{get_ability, get_actor, get_actor_mut},
};

// TODO:
// use smallvec to avoid heap fragmentation

pub use crate::action::{DestroyAbility, DestroyAbilityResponse};

impl ActionInterface for DestroyAbility {
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
        let owner = ability.ownership_struct.owner;
        let pool_ids: Vec<ChargePoolKey> = ability
            .pool_links
            .iter()
            .map(|l| l.link.link_dest)
            .collect();

        if let Some(owner_id) = owner {
            get_actor(eng, owner_id)?;
        }

        for pool_id in pool_ids {
            Action::RemoveLink(RemoveLink {
                ability_id: self.ability_id,
                pool_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        let bug_ids: Vec<BugKey> = eng
            .world
            .bugs
            .iter()
            .filter(|(_, bug)| BugSource::Ability(self.ability_id) == bug.source)
            .map(|(id, _)| id)
            .collect();

        for bug_id in bug_ids {
            Action::DestroyBug(DestroyBug { bug_id }).handle(eng, ctx, actor, version, mutate)?;
        }

        if mutate {
            if let Some(owner_id) = owner {
                get_actor_mut(eng, owner_id)
                    .expect("ability owner does not exist: engine invariant violated")
                    .remove_ability(self.ability_id);
            }
            eng.world.remove_ability(self.ability_id);
        }

        Action::CullKidnappings(CullKidnappings { ability_id: self.ability_id })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Action::CullIncarcerations(CullIncarcerations { ability_id: self.ability_id })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(ActionResponse::DestroyAbility(DestroyAbilityResponse {}))
    }
}
