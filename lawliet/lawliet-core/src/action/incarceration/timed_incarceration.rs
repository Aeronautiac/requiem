/*
* SYSTEM ACTION
* Incarcerate a player and schedule their automatic release after `duration`.
*
* Bundles CreateIncarceration with a ScheduleJob(ReleaseIncarceration) so the whole
* "arrest for a while, then let go" effect is a single action — needed because a poll's
* accept payload is one action (this is the civilian arrest vote's accept payload).
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        CreateIncarceration, ReleaseIncarceration, ScheduleJob,
    },
    common::Version,
    engine::Engine,
};

pub use crate::action::{TimedIncarceration, TimedIncarcerationResponse};

impl ActionInterface for TimedIncarceration {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let response = Action::CreateIncarceration(CreateIncarceration {
            victim_id: self.victim_id,
            source: self.source,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        let ActionResponse::CreateIncarceration(data) = response else {
            unreachable!()
        };

        Action::ScheduleJob(ScheduleJob {
            payload: Box::new(Action::ReleaseIncarceration(ReleaseIncarceration {
                incarceration_id: data.id,
                forced: false,
            })),
            timestamp: eng.time + self.duration,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(ActionResponse::TimedIncarceration(
            TimedIncarcerationResponse {},
        ))
    }
}
