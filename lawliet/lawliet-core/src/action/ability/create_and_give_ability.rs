/*
* SYSTEM ACTION
* Atomically create an ability and give it to an actor
*/

use crate::action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, AddAbility, GiveAbility,
    };

pub use crate::action::{CreateAndGiveAbility, CreateAndGiveAbilityResponse};

impl ActionInterface for CreateAndGiveAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let add_response = Action::AddAbility(AddAbility {
            ability_name: self.ability_name,
            variant: self.variant,
            transferrable: self.transferrable,
        })
        .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::AddAbility(add_response_data) = add_response else {
            // if it returns the wrong struct, then the engine is broken, and a crash is
            // warranted
            unreachable!()
        };

        if mutate {
            Action::GiveAbility(GiveAbility {
                ability_id: add_response_data.id,
                actor_id: self.actor_id,
                volatile: self.volatile,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::CreateAndGiveAbility(
            CreateAndGiveAbilityResponse {
                id: add_response_data.id,
            },
        ))
    }
}
