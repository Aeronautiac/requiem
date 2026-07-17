/*
* SYSTEM ACTION
* Schedule a revive action
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionRequest, ActionResponse,
        ActionResult,
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
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
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
