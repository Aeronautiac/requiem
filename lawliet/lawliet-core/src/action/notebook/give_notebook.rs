/*
* SYSTEM ACTION
* Give a player true ownership of a notebook
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, SetNotebookPossession,
    },
    common::Version,
    engine::Engine,
    helpers::{cmd_notebook_fake_status, get_notebook, get_notebook_mut, require_player},
};

pub use crate::action::{GiveNotebook, GiveNotebookResponse};

impl ActionInterface for GiveNotebook {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        require_player(eng, self.actor_id)?;

        let notebook = get_notebook(eng, self.notebook_id)?;
        if let Some(owner) = notebook.get_true_owner() {
            let true_owner = owner;
            if true_owner == self.actor_id && notebook.owner == Some(self.actor_id) {
                return Err(ActionError::ItemAlreadyOwned);
            }
        }

        let old_holder = get_notebook(eng, self.notebook_id)?.owner;

        // This give establishes the original owner iff there isn't one yet — set_true_owner only
        // sets it while it is None. That is the one moment the fake status is stated to them, below.
        let establishes_owner = get_notebook(eng, self.notebook_id)?
            .original_owner
            .is_none();

        let notebook = get_notebook_mut(eng, self.notebook_id)?;
        if mutate {
            notebook.set_true_owner(self.actor_id, self.volatile);
        }

        // Transfer possession: remove old holder, give to new owner.
        // SetNotebookPossession handles actor cache + channel perms.
        let from = if old_holder != Some(self.actor_id) {
            old_holder
        } else {
            None
        };
        Action::SetNotebookPossession(SetNotebookPossession {
            notebook_id: self.notebook_id,
            from,
            to: Some(self.actor_id),
        })
        .handle(eng, ctx, actor, version, mutate)?;

        // Stated after the owner is seated in the notebook channel, and only on the give that
        // establishes them. A later change to the flag restates it through SetNotebookFake.
        if establishes_owner {
            cmd_notebook_fake_status(eng, ctx, self.notebook_id);
        }

        Ok(ActionResponse::GiveNotebook(GiveNotebookResponse {}))
    }
}
