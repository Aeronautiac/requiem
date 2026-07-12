// TODO:
// just create a public kidnapping and schedule a release

use lawliet_types::{
    ability::{AbilityName, PublicKidnap},
    action::{
        Action, ActionActor, ActionResponse, CreateKidnapping, ReleaseKidnapping, ScheduleJob,
    },
    kidnapping::{KidnappingSource, KidnappingType},
};

use crate::{
    ability::AbilityInterface,
    action::ActionInterface,
    helpers::{actor_id, get_player},
};

impl AbilityInterface for PublicKidnap {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::PublicKidnap
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
        let id = actor_id(actor).expect("expected valid actor to use ability");

        let response = Action::CreateKidnapping(CreateKidnapping {
            victim_id: self.target,
            kidnapping_type: KidnappingType::Public(lawliet_types::actor::ActorDisplay::Raw(id)),
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

        Ok(super::AbilityStatus::Success)
    }
}
