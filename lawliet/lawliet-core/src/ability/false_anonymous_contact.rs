// Pose as another role: identical to AnonymousContact, but instead of surfacing the
// contactor's real role, it displays a role of their choosing to the contacted player.

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, comms::lounge::create_lounge::CreateLounge},
    config::ability::AbilityName,
    helpers::actor_id,
    lounge::LoungeVariant,
};

pub use lawliet_types::ability::FalseAnonymousContact;
use lawliet_types::lounge::AnonymousLoungeRoleDisplay;

impl AbilityInterface for FalseAnonymousContact {
    fn ability_name(&self) -> AbilityName {
        AbilityName::FalseAnonymousContact
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

        let id =
            actor_id(actor).expect("expected valid actor id within false anon contact ability");

        Action::CreateLounge(CreateLounge {
            variant: LoungeVariant::Anonymous {
                contacted_id: self.target,
                contactor_id: id,
                role_display: AnonymousLoungeRoleDisplay::Static(self.role),
            },
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
