/*
* SYSTEM ACTION
* Atomically create a notebook and give it to an actor
*/

use crate::action::{
    Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult, AddNotebook,
    GiveNotebook,
};

pub use crate::action::{CreateAndGiveNotebook, CreateAndGiveNotebookResponse};

impl ActionInterface for CreateAndGiveNotebook {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let add_response = Action::AddNotebook(AddNotebook { fake: self.fake })
            .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::AddNotebook(add_response_data) = add_response else {
            unreachable!()
        };

        if mutate {
            Action::GiveNotebook(GiveNotebook {
                notebook_id: add_response_data.id,
                actor_id: self.actor_id,
                volatile: self.volatile,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::CreateAndGiveNotebook(
            CreateAndGiveNotebookResponse {
                id: add_response_data.id,
            },
        ))
    }
}
