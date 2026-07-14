/*
* SYSTEM ACTION
* Keep game state up to date for anything that is fairly isolated but dependent
* on everything else in game and may in of itself influence game state
*/

pub use crate::action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, DeferredCmds, UpdatePolls, UpdateProsecutions,
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
        Action::UpdateProsecutions(UpdateProsecutions {}).handle(eng, ctx, actor, version, mutate)?;
        // DeferredCmds runs LAST: the update steps above queue this cycle's deferred commands
        // (e.g. a prosecution's UpdateProsecution broadcast). Flushing before them would hold
        // those commands until the next Update, so a freshly-started prosecution wouldn't reach
        // clients until some later action. Draining last delivers them in the same batch.
        Action::DeferredCmds(DeferredCmds {}).handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::Update(UpdateResponse {}))
    }
}
