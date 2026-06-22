/*
* SYSTEM ACTION
* Remove all links from an ability
*/

use crate::{
    action::{
        ActionInterface, Action, ActionResponse, RemoveLink,
    },
    common::AbilityKey,
    helpers::{get_ability_mut, get_charge_pool},
};

use crate::action::ActionActor;
pub use crate::action::{ClearLinks, ClearLinksResponse};

impl ActionInterface for ClearLinks {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let ability = get_ability_mut(eng, self.ability_id)?;
        let links = ability.pool_links.clone();
        for container in links {
            Action::RemoveLink(RemoveLink {
                ability_id: self.ability_id,
                pool_id: container.link.link_dest,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::ClearLinks(ClearLinksResponse {}))
    }
}
