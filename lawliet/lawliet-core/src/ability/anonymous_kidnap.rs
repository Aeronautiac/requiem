use lawliet_types::{
    ability::{AbilityName, AnonymousKidnap},
    action::{Action, ActionActor, Kidnap},
    kidnapping::{KidnappingSource, KidnappingType},
};

use crate::{ability::AbilityInterface, action::ActionInterface, helpers::get_player};

impl AbilityInterface for AnonymousKidnap {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::AnonymousKidnap
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        _actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        get_player(eng, self.target)?;

        Action::Kidnap(Kidnap {
            victim_id: self.target,
            kidnapping_type: KidnappingType::Anonymous,
            source: KidnappingSource::Ability(ability),
            duration: Some(eng.config.defaults.kidnap_time),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
