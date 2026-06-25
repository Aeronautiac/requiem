use crate::{
    ability::{AbilityInterface, AbilityResponse},
    action::{Action, ActionActor, ActionError, ActionInterface, comms::lounge::create_lounge::CreateLounge},
    actor::modifier::Modifier,
    config::ability::AbilityName,
    helpers::{actor_id, get_actor},
    lounge::LoungeVariant,
};

pub use lawliet_types::ability::{Contact, ContactResponse};

impl AbilityInterface for Contact {
    fn ability_name(&self) -> AbilityName {
        AbilityName::Contact
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        let contactor_id = actor_id(actor).expect("expected valid actor id within contact ability");

        if contactor_id == self.target_id {
            return Err(ActionError::CannotContactSelf);
        }

        let contactor_data = get_actor(eng, contactor_id)?;
        if contactor_data.has_modifier(Modifier::NoContact) {
            return Err(ActionError::CannotContact);
        }

        let target_data = get_actor(eng, self.target_id)?;
        if target_data.has_modifier(Modifier::NoContact) {
            return Err(ActionError::CannotContact);
        }

        Action::CreateLounge(CreateLounge {
            variant: LoungeVariant::Basic {
                contactor_id,
                contacted_id: self.target_id,
            },
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(AbilityResponse::Contact(ContactResponse {}))
    }
}
