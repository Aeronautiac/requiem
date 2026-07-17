/*
* SYSTEM ACTION
* Atomically create a passive and give it to an actor
*/

use crate::action::{
    Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult, AddPassive,
    GivePassive,
};

pub use crate::action::{CreateAndGivePassive, CreateAndGivePassiveResponse};

impl ActionInterface for CreateAndGivePassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let add_response = Action::AddPassive(AddPassive {
            passive_type: self.passive_type,
            transferrable: self.transferrable,
        })
        .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::AddPassive(add_response_data) = add_response else {
            unreachable!()
        };

        if mutate {
            Action::GivePassive(GivePassive {
                passive_id: add_response_data.id,
                actor_id: self.actor_id,
                volatile: self.volatile,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::CreateAndGivePassive(
            CreateAndGivePassiveResponse {
                id: add_response_data.id,
            },
        ))
    }
}
