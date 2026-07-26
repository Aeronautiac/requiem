/*
* SYSTEM ACTION
* Remove a poll from the world after it has concluded or been cancelled.
* Called by PollTimeout (natural conclusion), UpdatePolls (immediate resolution), and
* TerminateProsecution (cancellation). This is the single choke point for tearing a poll
* down, so it also emits the ClosePoll command that marks it concluded on the frontend.
*/

use indexmap::IndexSet;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    common::Version,
    engine::Engine,
    helpers::{get_poll, sync_viewport},
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
        let viewport = get_poll(eng, self.poll_id)?.viewport;

        // Announce the outcome while the viewers can still receive it, then empty the viewport,
        // then free it. The poll closes rather than disappearing: whoever could see it keeps
        // the concluded poll and its result.
        ctx.push_cmd(
            Command::ClosePoll {
                poll_id: self.poll_id,
                outcome: self.outcome,
            },
            CommandRecipient::Viewport(viewport),
            eng.time,
        );

        sync_viewport(eng, ctx, viewport, IndexSet::new(), mutate);

        if mutate {
            eng.world.remove_poll(self.poll_id);
            eng.world.remove_viewport(viewport);
        }

        Ok(ActionResponse::PollCleanup(PollCleanupResponse {}))
    }
}
