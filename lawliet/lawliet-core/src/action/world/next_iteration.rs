use lawliet_types::{
    action::{
        Action, ActionActor, ActionError, ActionRequest, ActionResponse, ArchiveBug, NextIteration,
        ReturnBorrowedNotebooks,
    },
    actor::State,
    bug::BugSource,
    command::Command,
    common::BugKey,
};
use smallvec::SmallVec;

use crate::{action::ActionInterface, helpers::cmd_world_data};

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

        if actor.is_admin() && !eng.initialized {
            return Err(ActionError::EngineNotInitialized);
        }

        if mutate {
            for (_, notebook) in eng.world.notebooks.iter_mut() {
                notebook.iteration_reset();
            }

            for (_, pool) in eng.world.charge_pools.iter_mut() {
                pool.on_iteration();
            }

            // Iteration-scoped states: they last until the boundary and no further, so there is
            // nothing per-actor to count down.
            for (_, actor) in eng.world.actors.iter_mut() {
                actor.remove_state(State::Ipp);
                actor.remove_state(State::UnderTheRadar);
            }

            eng.world.curr_iteration += 1;
        }

        // Planted bugs last a day. A custody wiretap is not planted and does not expire with one —
        // it belongs to the custody, and lasts exactly as long as the prosecution keeps its
        // defendant there. SetCustody is the only thing that ends it.
        let keys: SmallVec<[BugKey; 8]> = eng
            .world
            .bugs
            .iter()
            .filter_map(|(id, bug)| matches!(bug.source, BugSource::Ability(_)).then_some(id))
            .collect();
        for id in keys {
            Action::ArchiveBug(ArchiveBug { bug_id: id })
                .handle(eng, ctx, actor, version, mutate)?;
        }

        Action::ReturnBorrowedNotebooks(ReturnBorrowedNotebooks {}).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;

        // Re-arm, or leave the clock alone if the host owns it.
        //
        // The pending advance is cancelled first, which is what makes an early manual turn behave:
        // the new day gets a full duration instead of being cut short by a timer belonging to the
        // day before. A natural turn cancels a job that has already fired, which is a no-op.
        if mutate {
            if let Some(job) = eng.world.iteration_job.take() {
                eng.jobs.cancel_id(job);
            }
            if eng.config.defaults.iterations_autonomous {
                let at = eng.time + eng.config.defaults.iteration_duration;
                let job = eng.jobs.push(ActionRequest {
                    actor: ActionActor::System,
                    timestamp: at,
                    payload: Action::NextIteration(NextIteration {}),
                });
                eng.world.iteration_job = Some(job);
            }
        }

        // World DATA: the clock is not an announcement. A prisoner and a blacked-out world both
        // still need to know what day it is, or nothing else they can see makes sense.
        cmd_world_data(
            eng,
            ctx,
            Command::NewIteration {
                iteration: eng.world.curr_iteration,
            },
        );

        Ok(ActionResponse::NextIteration(
            lawliet_types::action::NextIterationResponse {},
        ))
    }
}
