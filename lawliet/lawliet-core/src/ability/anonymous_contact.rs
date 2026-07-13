// TODO:
// potentially add an option to tie lounges to ability ids. if the ability is destroyed, so is the lounge.
// another approach is to switch it to a normal lounge.
// do not refund the contact token.
// also add dynamic role tracking for the case where the anonymous lounge persists after a role change.
// this is low priority right now.

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, comms::lounge::create_lounge::CreateLounge},
    config::ability::AbilityName,
    helpers::{actor_id, get_player},
    lounge::LoungeVariant,
};

pub use lawliet_types::ability::AnonymousContact;
use lawliet_types::{action::ActionError, lounge::AnonymousLoungeRoleDisplay};

impl AbilityInterface for AnonymousContact {
    fn ability_name(&self) -> crate::config::ability::AbilityName {
        AbilityName::AnonymousContact
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
        actor.player_only()?;

        let id = actor_id(actor).expect("expected valid actor id within anon contact ability");
        let role = get_player(eng, id)?.role;

        if self.target == id {
            return Err(ActionError::CannotContactSelf);
        }

        Action::CreateLounge(CreateLounge {
            variant: LoungeVariant::Anonymous {
                contacted_id: self.target,
                contactor_id: id,
                role_display: AnonymousLoungeRoleDisplay::Static(role),
            },
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
