/*
* SYSTEM ACTION
* Go through every book with a specific true owner and set the dormant true owner to that person
*/

use crate::action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    };

pub use crate::action::{SetBooksDormant, SetBooksDormantResponse};

impl ActionInterface for SetBooksDormant {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        if mutate {
            for notebook in eng.world.notebooks.values_mut() {
                if notebook.get_true_owner() == Some(self.actor_id) {
                    notebook.set_dormant();
                }
            }
        }

        Ok(ActionResponse::SetBooksDormant(SetBooksDormantResponse {}))
    }
}
