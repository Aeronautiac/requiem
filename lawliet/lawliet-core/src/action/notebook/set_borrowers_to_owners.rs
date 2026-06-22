/*
* SYSTEM ACTION
* Go through all notebooks. If a notebook's true owner is the target actor's, and the notebook is
* being borrowed, then set the borrower as the true owner.
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, GiveNotebook,
    },
    common::ActorKey,
    helpers::get_actor,
};

pub use crate::action::{SetBorrowersToOwners, SetBorrowersToOwnersResponse};

impl ActionInterface for SetBorrowersToOwners {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        get_actor(eng, self.actor_id)?;

        let mut next_actions = vec![];
        for (id, notebook) in eng.world.notebooks.iter() {
            if notebook.get_true_owner() == Some(self.actor_id) && notebook.is_owner_borrowing() {
                next_actions.push(Action::GiveNotebook(GiveNotebook {
                    notebook_id: id,
                    actor_id: notebook.owner.unwrap(),
                    volatile: false,
                }));
            }
        }
        for mut action in next_actions {
            action.handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::SetBorrowersToOwners(
            SetBorrowersToOwnersResponse {},
        ))
    }
}
