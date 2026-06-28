use lawliet_types::{
    action::{Action, ActionResponse, ArchiveBug, NextIteration},
    actor::State,
    common::BugKey,
};
use smallvec::SmallVec;

use crate::action::ActionInterface;

impl ActionInterface for NextIteration {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        version: lawliet_types::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        if mutate {
            for (_, notebook) in eng.world.notebooks.iter_mut() {
                notebook.iteration_reset();
            }

            for (_, pool) in eng.world.charge_pools.iter_mut() {
                pool.on_iteration();
            }

            for (_, actor) in eng.world.actors.iter_mut() {
                actor.remove_state(State::Ipp);
            }

            eng.world.curr_iteration += 1;
        }

        let keys: SmallVec<[BugKey; 8]> = eng.world.bugs.keys().collect();
        for id in keys {
            Action::ArchiveBug(ArchiveBug { bug_id: id })
                .handle(eng, ctx, actor, version, mutate)?;
        }

        // TODO:
        // announce new iteration

        Ok(ActionResponse::NextIteration(
            lawliet_types::action::NextIterationResponse {},
        ))
    }
}
