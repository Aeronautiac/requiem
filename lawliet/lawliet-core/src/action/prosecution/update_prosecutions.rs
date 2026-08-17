/*
* SYSTEM ACTION
* The unified prosecution update, run from the global Update step after every action. It keeps
* everything prosecution-related consistent in one place:
*   1. cull prosecutions whose invariants broke (may terminate + emit CloseProsecution)
*   2. for every surviving prosecution: advance it if both sides have signalled, re-evaluate trial
*      channel perms, then broadcast its client-facing snapshot
*
* Because Update runs after every action (only on the execute pass), this also covers state
* changes that aren't prosecution actions — e.g. a spectator gaining/losing presence re-runs the
* channel eval without any prosecution-specific trigger.
*
* The advance lives here rather than in SignalReady for the same reason. Both sides having
* signalled is a standing fact, and the moment it becomes true is not necessarily a moment the
* prosecution can act on it: a frozen one cannot move at all. Re-reading it every sweep is what
* makes the trial resume by itself when the lights come back on, with nothing having to remember
* that it was owed an advance.
*/

use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        AdvanceProsecution, CullProsecutions,
    },
    common::{ProsecutionKey, Version},
    engine::Engine,
};

use super::broadcast_prosecution;

pub use crate::action::{UpdateProsecutions, UpdateProsecutionsResponse};

impl ActionInterface for UpdateProsecutions {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        loop {
            // Cull first so terminated prosecutions are gone before we refresh the survivors.
            Action::CullProsecutions(CullProsecutions {}).handle(
                eng,
                ctx,
                &ActionActor::System,
                version,
                mutate,
            )?;

            let mut one_advanced = false;
            let ids: SmallVec<[ProsecutionKey; 8]> = eng.world.prosecutions.keys().collect();
            for prosecution_id in ids {
                // Held prosecutions are skipped rather than refused: a hold is already recorded, and
                // asking again every sweep would only rewrite it. A frozen one is not skipped here —
                // AdvanceProsecution refuses it, and that refusal leaves nothing behind, so the sweep
                // after the freeze lifts is the one that carries.
                let advance = eng
                    .world
                    .get_prosecution(prosecution_id)
                    .is_some_and(|p| p.both_signalled() && !p.pending_advance);

                if advance {
                    one_advanced = true;
                    Action::AdvanceProsecution(AdvanceProsecution { prosecution_id }).handle(
                        eng,
                        ctx,
                        &ActionActor::System,
                        version,
                        mutate,
                    )?;
                }

                broadcast_prosecution(eng, ctx, prosecution_id, mutate);
            }

            if !one_advanced {
                break;
            }
        }

        Ok(ActionResponse::UpdateProsecutions(
            UpdateProsecutionsResponse {},
        ))
    }
}
