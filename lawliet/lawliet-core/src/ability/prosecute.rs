use lawliet_types::{
    ability::{AbilityName, Prosecute},
    action::{Action, ActionActor, StartProsecution},
    actor::ActorDisplay,
    prosecution::ProsecutionSource,
};

use crate::{
    ability::AbilityInterface,
    action::ActionInterface,
    helpers::{actor_id, get_player},
};

impl AbilityInterface for Prosecute {
    fn ability_name(&self) -> AbilityName {
        AbilityName::Prosecute
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        let prosecutor_id =
            actor_id(actor).expect("expected valid actor to have prosecute ability");
        // Validate the prosecutor is a player (StartProsecution re-checks, but the Raw
        // display below is only meaningful for a player).
        get_player(eng, prosecutor_id)?;

        Action::StartProsecution(StartProsecution {
            autonomous: eng.config.defaults.prosecution_autonomous,
            defendant_id: self.target,
            source: ProsecutionSource::Ability(ability),
            defendant_display: ActorDisplay::Raw(self.target),
            prosecutor_display: ActorDisplay::Raw(prosecutor_id),
            prosecutor_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
