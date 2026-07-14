use crate::action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    };

pub use crate::action::{Null, NullResponse};

impl ActionInterface for Null {
    fn handle(
        &mut self,
        _eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
        _actor: &ActionActor,
        _version: crate::common::Version,
        _mutate: bool,
    ) -> ActionResult {
        Ok(ActionResponse::Null(NullResponse {}))
    }
}
