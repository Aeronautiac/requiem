// Contact someone while showing only your role.
//
// TODO:
// potentially add an option to tie lounges to ability ids. if the ability is destroyed, so is the lounge.
// another approach is to switch it to a normal lounge.
// do not refund the contact token.
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
        version: u64,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.player_only()?;

        let id = actor_id(actor).expect("expected valid actor id within anon contact ability");
        let role = get_player(eng, id)?.role;

        if self.target == id {
            return Err(ActionError::CannotContactSelf);
        }

        // Static: the role is read once, here, and the lounge keeps showing it afterwards. Switch
        // to Dynamic once a role change can update a display that already exists — until then
        // Dynamic would be resolved once at creation anyway (see create_lounge) and only look
        // like it tracked anything.
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
