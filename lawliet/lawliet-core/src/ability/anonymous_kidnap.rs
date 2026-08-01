use lawliet_types::{
    ability::{AbilityName, AnonymousKidnap},
    action::{Action, ActionActor, ActionError, CreateKidnapping},
    kidnapping::{KidnappingSource, KidnappingType},
};

use crate::{
    ability::AbilityInterface,
    action::ActionInterface,
    helpers::{actor_id, get_player},
};

impl AbilityInterface for AnonymousKidnap {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::AnonymousKidnap
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
        get_player(eng, self.target)?;

        let id = actor_id(actor).expect("expected valid actor id within anon kidnap ability");
        if self.target == id {
            return Err(ActionError::CannotTargetSelf);
        }

        Action::CreateKidnapping(CreateKidnapping {
            victim_id: self.target,
            kidnapping_type: KidnappingType::Anonymous,
            source: KidnappingSource::Ability(ability),
            duration: Some(eng.config.defaults.kidnap_time),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
