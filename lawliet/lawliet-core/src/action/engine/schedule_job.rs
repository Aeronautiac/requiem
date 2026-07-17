/*
* SYSTEM ACTION
* Schedule a job
*/

use crate::{
    action::{
        ActionActor, ActionContext, ActionInterface, ActionRequest, ActionResponse, ActionResult,
    },
    common::Version,
    engine::Engine,
    helpers::require_time_not_passed,
};

pub use crate::action::{ScheduleJob, ScheduleJobResponse};

impl ActionInterface for ScheduleJob {
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
            eng.schedule(ActionRequest {
                actor: actor.clone(),
                timestamp: self.timestamp,
                payload: *self.payload.clone(),
            });
        }

        Ok(ActionResponse::ScheduleJob(ScheduleJobResponse {}))
    }
}
