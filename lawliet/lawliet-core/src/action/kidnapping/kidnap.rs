/*
* SYSTEM / ADMIN ACTION
* Orchestrate a kidnapping over the low-level primitives: create the kidnapping (object +
* channel + state), announce it, and — when `duration` is Some — schedule the automatic
* release. Mirrors TimedIncarceration; abilities go through this instead of hand-rolling
* CreateKidnapping + ScheduleJob. `duration` None is an indefinite kidnapping (no auto-release).
*
* The Kidnapping announce is emitted HERE, not in create_kidnapping, because the orchestrator
* is the layer that knows the duration; create_kidnapping stays a pure low-level primitive.
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        CreateKidnapping, ReleaseKidnapping, ScheduleJob,
    },
    actor::modifier::Modifier,
    command::Command,
    common::Version,
    engine::Engine,
    helpers::cmd_all_deferred,
};

pub use crate::action::{Kidnap, KidnapResponse};

impl ActionInterface for Kidnap {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let response = Action::CreateKidnapping(CreateKidnapping {
            victim_id: self.victim_id,
            kidnapping_type: self.kidnapping_type,
            source: self.source,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        let ActionResponse::CreateKidnapping(data) = response else {
            unreachable!()
        };
        let id = data.id;

        // Announce the kidnapping (victim public from the start). NoPresence players don't
        // receive world events; System and Base see it unconditionally.
        cmd_all_deferred(
            eng,
            ctx,
            Command::Kidnapping {
                kidnapping_id: id,
                target_id: self.victim_id,
                duration: self.duration,
            },
            Modifier::NoPresence.into(),
            true,
            true,
            mutate,
        );

        if let Some(duration) = self.duration {
            Action::ScheduleJob(ScheduleJob {
                payload: Box::new(Action::ReleaseKidnapping(ReleaseKidnapping {
                    kidnapping_id: id,
                    forced: false,
                })),
                timestamp: eng.time + duration,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::Kidnap(KidnapResponse {}))
    }
}
