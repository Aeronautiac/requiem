/*
* SYSTEM ACTION
* Create the world's base organizations
*/

use crate::action::ActionActor;
pub use crate::action::{
        ActionInterface, ActionResponse,
    };

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

        // TODO:
        // implement it

        Ok(ActionResponse::CreateOrgs(CreateOrgsResponse {}))
    }
}

