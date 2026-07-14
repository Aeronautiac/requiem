/*
* SYSTEM ACTION
* A kill wrapper used to differentiate between notebook scheduled kill jobs and host/system
* scheduled jobs
* This can potentially hold metadata in the future such as the ID for the notebook which scheduled
* the job
*/

pub use crate::action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse,
    };

pub use crate::action::{NotebookScheduledKill, NotebookScheduledKillResponse};

impl ActionInterface for NotebookScheduledKill {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        Action::Kill(self.kill.clone()).handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::NotebookScheduledKill(
            NotebookScheduledKillResponse {},
        ))
    }
}
