/*
* SYSTEM ACTION
* Give a player true ownership of a notebook
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionError, ActionResponse,
    },
    common::{ActorKey, NotebookKey, Version},
    engine::Engine,
    helpers::{get_actor_mut, get_notebook, get_notebook_mut, require_player},
};

pub use crate::action::{GiveNotebook, GiveNotebookResponse};

impl ActionInterface for GiveNotebook {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        require_player(eng, self.actor_id)?;

        // Semantics:
        // - If the notebook is currently held by the actor, and the actor is already the true owner
        // of the notebook, then this action will do nothing and an error should be returned.
        // - If the notebook is currently held by someone, but the true owner is NOT the actor,
        // then true ownership should transfer to the actor.
        // - If the actor is the true owner, but the notebook is held by someone else, then the
        // notebook should be sent back to the actor.

        let notebook = get_notebook(eng, self.notebook_id)?;
        if let Some(owner) = notebook.owner {
            let true_owner = notebook.get_true_owner().unwrap();
            if true_owner == self.actor_id && owner == self.actor_id {
                return Err(ActionError::ItemAlreadyOwned);
            }
            if mutate && owner != self.actor_id {
                let other_actor = get_actor_mut(eng, owner).unwrap(); // if
                // the owner doesn't exist, there's something wrong with the engine
                other_actor.remove_notebook(self.notebook_id); // removes from the actor's cache
            }
        }

        let notebook = get_notebook_mut(eng, self.notebook_id)?;
        if mutate {
            notebook.set_true_owner(self.actor_id, self.volatile);
            let actor = get_actor_mut(eng, self.actor_id)?;
            actor.add_notebook(self.notebook_id);
        }

        Ok(ActionResponse::GiveNotebook(GiveNotebookResponse {}))
    }
}
