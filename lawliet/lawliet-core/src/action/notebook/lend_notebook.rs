/*
* PLAYER ACTION
* Lend a notebook to another player
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionError, ActionResponse,
    },
    actor::modifier::Modifier,
    common::{ActorKey, NotebookKey, Version},
    engine::Engine,
    helpers::{actor_id, get_actor_mut, get_notebook_mut},
};

pub use crate::action::{LendNotebook, LendNotebookResponse};

impl ActionInterface for LendNotebook {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.player_only()?;

        let user_id = actor_id(actor).unwrap();
        if user_id == self.target_id {
            return Err(ActionError::CannotLendToYourself);
        }

        let notebook = get_notebook_mut(eng, self.notebook_id)?;
        if notebook.can_lend(user_id).is_err() {
            return Err(ActionError::NotebookNotOwned);
        }
        if mutate {
            notebook.lend(self.target_id).unwrap();
        }

        let player_actor = get_actor_mut(eng, user_id)?;
        if player_actor.has_modifier(Modifier::NoNotebookPassage) {
            return Err(ActionError::NotebookPassageBlocked);
        }
        if mutate {
            player_actor.remove_notebook(self.notebook_id);
        }

        let target_actor = get_actor_mut(eng, self.target_id)?;
        if target_actor.has_modifier(Modifier::NoNotebookReceive) {
            return Err(ActionError::ActorHasNotebookReceiveRestriction);
        }
        if mutate {
            target_actor.add_notebook(self.notebook_id);
        }

        Ok(ActionResponse::LendNotebook(LendNotebookResponse {}))
    }
}
