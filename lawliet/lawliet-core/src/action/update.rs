/*
* SYSTEM ACTION
* Keep game state up to date for anything that is fairly isolated but dependent
* on everything else in game and may in of itself influence game state
*/

pub use crate::action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, DeferredCmds, UpdatePolls, CullProsecutions,
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

        Action::DeferredCmds(DeferredCmds {}).handle(eng, ctx, actor, version, mutate)?;
        Action::UpdatePolls(UpdatePolls {}).handle(eng, ctx, actor, version, mutate)?;
        Action::CullProsecutions(CullProsecutions {}).handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::Update(UpdateResponse {}))
    }
}
