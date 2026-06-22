/*
* SYSTEM ACTION
* Take a notebook away from someone (set the owner to None)
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionError, ActionResponse,
    },
    common::NotebookKey,
    helpers::{get_actor_mut, get_notebook, get_notebook_mut},
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
        if mutate {
            if let Some(owner) = notebook.owner {
                let owner_actor = get_actor_mut(eng, owner).unwrap();
                owner_actor.remove_notebook(self.notebook_id);
            }
            let notebook = get_notebook_mut(eng, self.notebook_id)?;
            notebook.strip_ownership();
        }

        Ok(ActionResponse::TakeNotebook(TakeNotebookResponse {}))
    }
}
