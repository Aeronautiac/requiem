/*
* SYSTEM ACTION
* Schedule a kill action
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionRequest, ActionResponse, NotebookScheduledKill,
    },
    common::Version,
    engine::Engine,
    helpers::require_time_not_passed,
};

pub use crate::action::{ScheduleKill, ScheduleKillResponse};

impl ActionInterface for ScheduleKill {
    fn handle(
        &mut self,
        eng: &mut Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        require_time_not_passed(eng, self.timestamp)?;

        if mutate {
            if self.notebook_scheduled {
                eng.schedule(ActionRequest {
                    actor: ActionActor::System,
                    timestamp: self.timestamp,
                    payload: Action::NotebookScheduledKill(NotebookScheduledKill {
                        kill: self.kill.clone(),
                    }),
                })
            } else {
                eng.schedule(ActionRequest {
                    actor: ActionActor::System,
                    timestamp: self.timestamp,
                    payload: Action::Kill(self.kill.clone()),
                })
            }
        }

        Ok(ActionResponse::ScheduleKill(ScheduleKillResponse {}))
    }
}
