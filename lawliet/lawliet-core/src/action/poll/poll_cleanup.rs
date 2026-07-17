/*
* SYSTEM ACTION
* Remove a poll from the world after it has concluded or been cancelled.
* Called by PollTimeout (natural conclusion), UpdatePolls (immediate resolution), and
* TerminateProsecution (cancellation). This is the single choke point for tearing a poll
* down, so it also emits the ClosePoll command that drops it on the frontend.
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
        get_poll(eng, self.poll_id)?;

        if mutate {
            ctx.push_cmd(
                Command::ClosePoll {
                    poll_id: self.poll_id,
                    outcome: self.outcome,
                },
                CommandRecipient::System,
                eng.time,
            );
            eng.world.remove_poll(self.poll_id);
        }

        Ok(ActionResponse::PollCleanup(PollCleanupResponse {}))
    }
}
