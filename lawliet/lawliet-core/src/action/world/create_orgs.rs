/*
* SYSTEM ACTION
* Create the world's base organizations
*/

use crate::action::{Action, ActionActor, ActionInterface, ActionResponse, CreateOrg};

pub use crate::action::{CreateOrgs, CreateOrgsResponse};

impl ActionInterface for CreateOrgs {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let orgs = eng.config.world_config.default_orgs.clone();
        for name in orgs {
            Action::CreateOrg(CreateOrg { name }).handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::CreateOrgs(CreateOrgsResponse {}))
    }
}

