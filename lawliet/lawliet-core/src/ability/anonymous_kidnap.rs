use lawliet_types::{
    ability::{AbilityName, AnonymousKidnap},
    action::{
        Action, ActionActor, ActionResponse, CreateKidnapping, ReleaseKidnapping, ScheduleJob,
    },
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

        let response = Action::CreateKidnapping(CreateKidnapping {
            victim_id: self.target,
            kidnapping_type: KidnappingType::Anonymous,
            source: KidnappingSource::Ability(ability),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        let ActionResponse::CreateKidnapping(data) = response else {
            unreachable!()
        };
        let id = data.id;

        let duration = eng.config.defaults.kidnap_time;
        let expiry_time = eng.time + duration;
        Action::ScheduleJob(ScheduleJob {
            payload: Box::new(Action::ReleaseKidnapping(ReleaseKidnapping {
                kidnapping_id: id,
                forced: false,
            })),
            timestamp: expiry_time,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(())
    }
}
