/*
* SYSTEM ACTION
* Decide whether time is running.
*
* A timer runs while the audience it was given to can still see. Losing them does not shorten it
* and does not cancel it: the clock stops and the time is handed back when they return. Waiting
* out a vote in the dark is not the same as waiting it out in the light.
*
* Every reason an audience can be taken away ends in that audience's viewport emptying — a
* blackout draining world events, a channel losing its permissions for everyone, the last member
* of an org leaving. So this asks one question, about the outcome, and never grows a branch per
* cause.
*
* One loop over every timer in the world, and it stays that way. It has no idea what any of them
* are counting down for — that is the whole reason timers are objects rather than fields (see the
* timer module), and the reason nothing needs adding here when the next kind of thing wants a
* pausable deadline.
*
* Recomputed rather than tracked, like the other sweeps Update trails with. Pause and resume are
* idempotent, so this converges on the right answer from any state — including a timer started
* while its audience was already gone, which is stopped on the way out of the very action that
* started it.
*/

use smallvec::SmallVec;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    common::{TimerKey, Version},
    engine::Engine,
};

pub use crate::action::{UpdateTimers, UpdateTimersResponse};

impl ActionInterface for UpdateTimers {
    fn handle(
        &mut self,
        eng: &mut Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        if mutate {
            let keys: SmallVec<[TimerKey; 8]> = eng.world.timers.keys().collect();
            for key in keys {
                let Some(gate) = eng.world.timers[key].gate else {
                    continue;
                };

                // A gate that no longer exists is one nobody is behind. The thing it belonged to
                // was torn down, and whatever owns this timer is about to notice.
                let watched = eng
                    .world
                    .get_viewport(gate)
                    .is_some_and(|viewport| !viewport.is_empty());

                let Engine {
                    world, jobs, time, ..
                } = eng;
                let timer = &mut world.timers[key];
                if watched {
                    timer.resume(jobs, *time);
                } else {
                    timer.pause(jobs, *time);
                }
            }
        }

        Ok(ActionResponse::UpdateTimers(UpdateTimersResponse {}))
    }
}
