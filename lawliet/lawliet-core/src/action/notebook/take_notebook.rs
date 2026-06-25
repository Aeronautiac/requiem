/*
* SYSTEM ACTION
* Take a notebook away from someone (set the owner to None)
*/

use crate::{
    action::{
        Action, ActionContext, ActionInterface, ActionResult, ActionActor, ActionError, ActionResponse,
        SetNotebookPossession,
    },
    helpers::{get_notebook, get_notebook_mut},
};

pub use crate::action::{TakeNotebook, TakeNotebookResponse};

impl ActionInterface for TakeNotebook {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let notebook = get_notebook(eng, self.notebook_id)?;
        if notebook.get_true_owner().is_none() {
            return Err(ActionError::ItemAlreadyUnowned);
        }
        let old_owner = notebook.owner;
        let old_lender = notebook.borrowed;

        if mutate {
            let notebook = get_notebook_mut(eng, self.notebook_id)?;
            notebook.strip_ownership();
        }

        // Remove both the current holder and the lender (if borrowed) from the channel.
        // SetNotebookPossession handles actor cache removal as well.
        if let Some(owner) = old_owner {
            Action::SetNotebookPossession(SetNotebookPossession {
                notebook_id: self.notebook_id,
                from: Some(owner),
                to: None,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }
        if let Some(lender) = old_lender {
            Action::SetNotebookPossession(SetNotebookPossession {
                notebook_id: self.notebook_id,
                from: Some(lender),
                to: None,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::TakeNotebook(TakeNotebookResponse {}))
    }
}
