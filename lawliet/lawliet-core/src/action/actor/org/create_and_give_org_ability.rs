/*
* SYSTEM ACTION
* Atomically create an ability and give it to an org
*/

use crate::action::{
        ActionInterface, Action, ActionResponse, AddAbility, GiveOrgAbility,
    };

use crate::action::ActionActor;
pub use crate::action::{CreateAndGiveOrgAbility, CreateAndGiveOrgAbilityResponse};

impl ActionInterface for CreateAndGiveOrgAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let add_response = Action::AddAbility(AddAbility {
            ability_name: self.ability_name,
            variant: self.variant,
            transferrable: false,
        })
        .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::AddAbility(add_response_data) = add_response else {
            unreachable!()
        };
        let id = add_response_data.id;

        if mutate {
            Action::GiveOrgAbility(GiveOrgAbility {
                org_id: self.org_id,
                ability_id: id,
                settings: self.settings.clone(),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::CreateAndGiveOrgAbility(
            CreateAndGiveOrgAbilityResponse { id },
        ))
    }
}
