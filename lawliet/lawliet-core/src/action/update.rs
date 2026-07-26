/*
* SYSTEM ACTION
* Keep game state up to date for anything that is fairly isolated but dependent
* on everything else in game and may in of itself influence game state
*/

pub use crate::action::{
    Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult, UpdatePolls,
    UpdateProsecutions,
};

pub use crate::action::{Update, UpdateResponse};

impl ActionInterface for Update {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        Action::UpdatePolls(UpdatePolls {}).handle(eng, ctx, actor, version, mutate)?;
        Action::UpdateProsecutions(UpdateProsecutions {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::Update(UpdateResponse {}))
    }
}
