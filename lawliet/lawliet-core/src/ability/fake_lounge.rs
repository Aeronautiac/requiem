// Fabricate a lounge: the creator is handed a lounge that reads as an ordinary
// basic lounge between two other players. Only the creator is a real participant,
// but they hold both players' displays, so they can author both sides of a
// conversation that never took place.

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, comms::lounge::create_lounge::CreateLounge},
    config::ability::AbilityName,
    helpers::{actor_id, get_player},
    lounge::LoungeVariant,
};

pub use lawliet_types::ability::FabricateLounge;

impl AbilityInterface for FabricateLounge {
    fn ability_name(&self) -> AbilityName {
        AbilityName::FabricateLounge
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

        let creator_id =
            actor_id(actor).expect("expected valid actor id within fabricate lounge ability");

        // The two players the fake conversation is attributed to must be real players —
        // create_lounge only validates the participant (creator) for the Fake variant, so
        // guard the displayed actors here.
        get_player(eng, self.contacted_id)?;
        get_player(eng, self.contactor_id)?;

        Action::CreateLounge(CreateLounge {
            variant: LoungeVariant::Fake {
                creator_id,
                contacted_id: self.contacted_id,
                contactor_id: self.contactor_id,
            },
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
