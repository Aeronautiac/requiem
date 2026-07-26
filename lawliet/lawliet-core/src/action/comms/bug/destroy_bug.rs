/*
* SYSTEM ACTION
* TODO: implement
*/

use indexmap::IndexSet;
use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    command::Command,
    helpers::{get_bug, get_player_mut, sync_viewport},
};

pub use crate::action::{DestroyBug, DestroyBugResponse};

impl ActionInterface for DestroyBug {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        let bug = get_bug(eng, self.bug_id)?;
        let (target_id, viewport) = (bug.target_id, bug.viewport);

        // Announce first, while the viewers are still in the viewport, then empty it, then
        // free it. This used to be DeleteBug — "this bug should never have existed" — which
        // monotonic state cannot honour: whoever was reading the relay has already read it.
        ctx.push_cmd(
            Command::ArchiveBug {
                bug_key: self.bug_id,
            },
            CommandRecipient::Viewport(viewport),
            eng.time,
        );

        sync_viewport(eng, ctx, viewport, IndexSet::new(), mutate);

        if mutate {
            get_player_mut(eng, target_id)
                .expect("expected valid player as a bug target")
                .remove_bug(self.bug_id);
            eng.world.remove_bug(self.bug_id);
            eng.world.remove_viewport(viewport);
        }

        Ok(ActionResponse::DestroyBug(DestroyBugResponse {}))
    }
}
