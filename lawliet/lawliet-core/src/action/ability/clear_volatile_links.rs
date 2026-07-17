/*
* SYSTEM ACTION
* Clear all of an ability's unowned links. Decrement charge pool reference count and try to destroy
* it if ref count hits zero.
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        RemoveLink,
    },
    helpers::get_ability_mut,
};

pub use crate::action::{ClearVolatileLinks, ClearVolatileLinksResponse};

impl ActionInterface for ClearVolatileLinks {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        let ability = get_ability_mut(eng, self.ability_id)?;

        let mut links_to_destroy = vec![];
        for link in &ability.pool_links {
            if link.volatile {
                links_to_destroy.push(link.link.link_dest);
            }
        }

        for id in links_to_destroy {
            Action::RemoveLink(RemoveLink {
                ability_id: self.ability_id,
                pool_id: id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::ClearVolatileLinks(
            ClearVolatileLinksResponse {},
        ))
    }
}
