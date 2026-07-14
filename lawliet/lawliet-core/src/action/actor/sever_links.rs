/*
* SYSTEM ACTION
* Sever every link to an actor ID
*/

use crate::action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    };

pub use crate::action::{SeverLinks, SeverLinksResponse};

impl ActionInterface for SeverLinks {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        _mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        for (_, actor) in eng.world.actors.iter_mut() {
            let links = actor.actor_links.clone();
            for link in links {
                if link.link_dest == self.actor_id {
                    actor.sever_link(link);
                }
            }
        }

        Ok(ActionResponse::SeverLinks(SeverLinksResponse {}))
    }
}
