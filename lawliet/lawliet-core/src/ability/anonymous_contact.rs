// TODO:
// create a lounge with a role display (for the contactor) and raw display for the contacted
// how to handle the case where someone anonymously contacts someone else, but then their role is
// changed?
// add an option to tie lounges to ability ids. if the ability is destroyed, so is the lounge.
// do not refund the contact token.

use crate::{
    ActorKey,
    ability::{AbilityInterface, AbilityResponse},
    action::{Action, ActionActor, ActionInterface, comms::lounge::create_lounge::CreateLounge},
    config::ability::AbilityName,
    helpers::actor_id,
    lounge::LoungeVariant,
};

pub use lawliet_types::ability::{AnonymousContact, AnonymousContactResponse};

impl AbilityInterface for AnonymousContact {
    fn ability_name(&self) -> crate::config::ability::AbilityName {
        AbilityName::AnonymousContact
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        ability: crate::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        let id = actor_id(actor).expect("expected valid actor id within anon contact ability");

        // TODO:
        // need to add a mechanism for anonymous lounges
        Action::CreateLounge(CreateLounge {
            variant: LoungeVariant::Basic {
                contacted_id: self.target,
                contactor_id: id,
            },
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(AbilityResponse::AnonymousContact(
            AnonymousContactResponse {},
        ))
    }
}
