/*
* SYSTEM ACTION
* Archive (disable) a bug.
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionInterface, ActionResponse},
    command::Command,
    helpers::get_bug_mut,
};

use crate::action::ActionActor;
pub use crate::action::{ArchiveBug, ArchiveBugResponse};

impl ActionInterface for ArchiveBug {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let bug = get_bug_mut(eng, self.bug_id)?;
        if mutate {
            bug.enabled = false;
        }

        ctx.push_cmd(
            Command::ArchiveBug {
                bug_key: self.bug_id,
            },
            CommandRecipient::System,
            eng.time,
        );

        Ok(ActionResponse::ArchiveBug(ArchiveBugResponse {}))
    }
}
