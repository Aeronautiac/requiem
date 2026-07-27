/*
* SYSTEM / ADMIN ACTION
* Kidnap a player: create the kidnapping object, channel, and apply State::Kidnapped.
*
* Preconditions:
* - victim exists, is a player, does not have NoPresence, does not have StrengthenedPresence
* - if source is Ability, that ability must exist
*
* On execution:
* - create channel (loggable)
* - AddState(victim, State::Kidnapped)
* - store Kidnapping in world
* - UpdateKidnapChannels (sets victim + ability-owner-side perms)
*
* TODO: commands
*/

use lawliet_types::command::Command;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, AddState, CreateChannel, ReleaseKidnapping, ScheduleJob,
        UpdateKidnapChannels,
    },
    actor::modifier::Modifier,
    actor::state::State,
    common::{KidnappingKey, Version},
    engine::Engine,
    helpers::{cmd_world_event, get_ability, get_actor, require_player},
    kidnapping::{Kidnapping, KidnappingSource},
};

pub use crate::action::{CreateKidnapping, CreateKidnappingResponse};

impl ActionInterface for CreateKidnapping {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        if let KidnappingSource::Ability(ab) = self.source {
            get_ability(eng, ab)?;
        }

        require_player(eng, self.victim_id)?;

        let victim = get_actor(eng, self.victim_id).expect("already validated");
        if victim.has_modifier(Modifier::NoPresence) {
            return Err(ActionError::UserNotPresent);
        }
        if victim.has_modifier(Modifier::StrengthenedPresence) {
            return Err(ActionError::ActorHasStrengthenedPresence);
        }

        let channel_response = Action::CreateChannel(CreateChannel { loggable: true }).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;
        let ActionResponse::CreateChannel(ch_data) = channel_response else {
            unreachable!()
        };
        let channel_id = ch_data.id;

        let id = if mutate {
            eng.world.add_kidnapping(Kidnapping {
                victim: self.victim_id,
                channel_id,
                kidnapping_type: self.kidnapping_type,
                source: self.source,
            })
        } else {
            KidnappingKey::default()
        };

        // Announced BEFORE the state change. Kidnapped carries NoPresence, which takes the victim
        // out of the very viewport this is addressed to — announcing afterwards tells everyone
        // except the person it happened to.
        cmd_world_event(
            eng,
            ctx,
            Command::Kidnapping {
                kidnapping_id: id,
                target_id: self.victim_id,
                duration: self.duration,
            },
        );

        Action::AddState(AddState {
            actor_id: self.victim_id,
            state: State::Kidnapped,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        if let Some(duration) = self.duration {
            Action::ScheduleJob(ScheduleJob {
                payload: Box::new(Action::ReleaseKidnapping(ReleaseKidnapping {
                    kidnapping_id: id,
                    forced: false,
                })),
                timestamp: eng.time + duration,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Action::UpdateKidnapChannels(UpdateKidnapChannels {}).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;

        Ok(ActionResponse::CreateKidnapping(CreateKidnappingResponse {
            id,
        }))
    }
}
