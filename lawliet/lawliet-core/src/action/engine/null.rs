use crate::action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    };

pub use crate::action::{Null, NullResponse};

impl ActionInterface for Null {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        Ok(ActionResponse::Null(NullResponse {}))
    }
}
