/*
* SYSTEM ACTION
* Fully destroy a notebook: remove from the current holder's actor cache, then remove from the world.
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        DestroyChannel,
    },
    helpers::{get_actor, get_actor_mut, get_notebook},
};

pub use crate::action::{DestroyNotebook, DestroyNotebookResponse};

impl ActionInterface for DestroyNotebook {
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
        let channel_id = notebook.channel_id;
        let owner = notebook.owner;

        if let Some(owner_id) = owner {
            get_actor(eng, owner_id)?;
        }

        Action::DestroyChannel(DestroyChannel { channel_id }).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;

        if mutate {
            if let Some(owner_id) = owner {
                get_actor_mut(eng, owner_id)
                    .expect("notebook owner does not exist: engine invariant violated")
                    .remove_notebook(self.notebook_id);
            }
            eng.world.remove_notebook(self.notebook_id);
        }

        Ok(ActionResponse::DestroyNotebook(DestroyNotebookResponse {}))
    }
}
