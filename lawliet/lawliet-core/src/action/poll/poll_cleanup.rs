/*
* SYSTEM ACTION
* Remove a poll from the world after it has concluded or been cancelled.
* Called by PollTimeout (natural conclusion), UpdatePolls (immediate resolution), and
* TerminateProsecution (cancellation). This is the single choke point for tearing a poll
* down, so it also emits the ClosePoll command that marks it concluded on the frontend.
*/

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    common::Version,
    engine::Engine,
    helpers::get_poll,
};
use lawliet_types::command::{Command, CommandRecipient};

pub use crate::action::{PollCleanup, PollCleanupResponse};

impl ActionInterface for PollCleanup {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        let poll = get_poll(eng, self.poll_id)?;
        let viewport = poll.viewport(eng);
        let timer = poll.timer;

        // The poll closes rather than disappearing: it rides its parent's viewport, so the
        // outcome stays in the parent's record alongside the vote that produced it.
        //
        // No viewport means the parent is already gone, and its record with it. There is nobody
        // left to tell, which is the whole reason this poll is being torn down.
        if let Some(viewport) = viewport {
            ctx.push_cmd(
                Command::ClosePoll {
                    poll_id: self.poll_id,
                    outcome: self.outcome,
                },
                CommandRecipient::Viewport(viewport),
                eng.time,
            );
        }

        if mutate {
            // Freeing the timer cancels its job, which is what matters for a poll resolved early
            // by its update policy: it still has a timeout queued, and that must not fire into
            // the gap the poll left behind. On the other path — the timeout itself getting here —
            // the job has already been popped and the cancel finds nothing, which is correct.
            if let Some(timer) = timer {
                let Engine { world, jobs, .. } = eng;
                world.remove_timer(timer, jobs);
            }
            eng.world.remove_poll(self.poll_id);
        }

        Ok(ActionResponse::PollCleanup(PollCleanupResponse {}))
    }
}
