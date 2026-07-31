/*
* SYSTEM ACTION
* Take the world dark, or bring it back.
*
* Sets the flag, announces the transition, and schedules the lift. What being dark actually MEANS
* is not spelled out here: the world-events viewport empties in UpdateWorldViewports and the
* blacked-out channels close in UpdateWorldChannels, both of which read the flag and both of which
* trail every action anyway. This action's job is the flag and the timer.
*
* Ending is a SetBlackout of its own, scheduled as a job, so blackout is the same action in both
* directions and there is no second path that could disagree with this one about what it means.
*/

use lawliet_types::{
    action::{Action, ActionRequest, SetBlackout},
    command::Command,
};

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    common::Version,
    engine::Engine,
    helpers::cmd_world_data,
};

pub use crate::action::SetBlackoutResponse;

impl ActionInterface for SetBlackout {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Read before the write. Re-arming a blackout that is already running would schedule a
        // second lift and announce a transition that never happened.
        let changed = eng.world.blackout != self.active;

        if mutate {
            eng.world.blackout = self.active;

            // Cancelled in both directions: ending early has to take the pending lift down with
            // it, or the world goes dark again when the old timer fires. A lift that has already
            // fired cancels a job that is no longer there, which is a no-op.
            if let Some(job) = eng.world.blackout_job.take() {
                eng.jobs.cancel_id(job);
            }
            if self.active {
                let at = eng.time + eng.config.defaults.blackout_duration;
                let job = eng.jobs.push(ActionRequest {
                    actor: ActionActor::System,
                    timestamp: at,
                    payload: Action::SetBlackout(SetBlackout { active: false }),
                });
                eng.world.blackout_job = Some(job);
            }
        }

        if changed {
            // Rides world data, so it survives the very blackout it announces. Pushed before the
            // viewport resync that follows in Update, so it reads in order on the way in: the
            // world goes dark, then the announcements stop.
            cmd_world_data(
                eng,
                ctx,
                Command::Blackout {
                    active: self.active,
                },
            );
        }

        Ok(ActionResponse::SetBlackout(SetBlackoutResponse {}))
    }
}
