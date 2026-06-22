/*
* SYSTEM ACTION
* Schedule a revive action
*/

use crate::{
    Time,
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionRequest, ActionResponse, Revive,
    },
    common::Version,
    engine::Engine,
    helpers::require_time_not_passed,
};

pub use crate::action::{ScheduleRevive, ScheduleReviveResponse};

impl ActionInterface for ScheduleRevive {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        require_time_not_passed(eng, self.timestamp)?;

        if mutate {
            eng.schedule(ActionRequest {
                actor: ActionActor::System,
                timestamp: self.timestamp,
                payload: Action::Revive(self.revive.clone()),
            })
        }

        Ok(ActionResponse::ScheduleRevive(ScheduleReviveResponse {}))
    }
}
