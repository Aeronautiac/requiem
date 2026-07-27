/*
* SYSTEM ACTION
* Sever every link to an actor ID
*/

use crate::action::{
    Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
    UpdatePassiveVisibilities,
};

pub use crate::action::{SeverLinks, SeverLinksResponse};

impl ActionInterface for SeverLinks {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        for (_, target) in eng.world.actors.iter_mut() {
            let links = target.actor_links.clone();
            for link in links {
                if link.link_dest == self.actor_id {
                    target.sever_link(link);
                }
            }
        }

        // A severed Passive link takes back whatever reach it granted.
        Action::UpdatePassiveVisibilities(UpdatePassiveVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::SeverLinks(SeverLinksResponse {}))
    }
}
