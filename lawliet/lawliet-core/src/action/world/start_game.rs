/*
* ADMIN / SYSTEM ACTION
* Begin play.
*
* Until this lands the world sits in Setup: it exists and is populated, players may talk in whatever
* channels they can already see, and nothing else. Explicit rather than automatic because setup is
* real work of no fixed length — the host is still building the roster, handing out roles and
* cutting keys, and none of that should be racing a day timer.
*
* The start IS the first turn of the clock, so it delegates to NextIteration rather than repeating
* it. That leaves exactly one thing in the engine that advances a day, whether the host fired it or
* a timer did, and the world opens on iteration 1 — iteration 0 being the time before play.
*/

use lawliet_types::{
    action::{Action, ActionError, ActionResponse, NextIteration},
    world::WorldPhase,
};

use crate::action::{ActionActor, ActionInterface};

pub use crate::action::{StartGame, StartGameResponse};

impl ActionInterface for StartGame {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        // Starting a game that is already running would turn a day early and re-arm the clock, so
        // it is refused rather than treated as a second advance.
        if eng.world.phase != WorldPhase::Setup {
            return Err(ActionError::GameAlreadyStarted);
        }

        // Before the delegation below, which is what everything downstream of it checks.
        if mutate {
            eng.world.phase = WorldPhase::Running;

            // only in the mutate pass because it refuses if not in a running state
            Action::NextIteration(NextIteration {}).handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::StartGame(StartGameResponse {}))
    }
}
