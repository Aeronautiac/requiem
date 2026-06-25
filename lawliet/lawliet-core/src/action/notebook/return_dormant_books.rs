/*
* SYSTEM ACTION
* Return all notebooks with dormant true owner equal to actor_id to the dormant true owner
*/

use crate::{
    action::{
        Action, ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
        SetNotebookPossession,
    },
    common::{ActorKey, NotebookKey},
};

pub use crate::action::{ReturnDormantBooks, ReturnDormantBooksResponse};

impl ActionInterface for ReturnDormantBooks {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Collect before mutating — we need &mut eng for SetNotebookPossession after.
        let affected: Vec<(NotebookKey, Option<ActorKey>)> = eng
            .world
            .notebooks
            .iter()
            .filter(|(_, nb)| nb.get_dormant_owner() == Some(self.actor_id))
            .map(|(id, nb)| (id, nb.owner))
            .collect();

        if mutate {
            for notebook in eng.world.notebooks.values_mut() {
                if notebook.get_dormant_owner() == Some(self.actor_id) {
                    notebook.awaken_dormant_owner();
                }
            }
        }

        for (notebook_id, prev_holder) in affected {
            Action::SetNotebookPossession(SetNotebookPossession {
                notebook_id,
                from: if prev_holder != Some(self.actor_id) { prev_holder } else { None },
                to: Some(self.actor_id),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::ReturnDormantBooks(ReturnDormantBooksResponse {}))
    }
}
