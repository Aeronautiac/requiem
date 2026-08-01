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
* - seat the victim in the channel
*/

use lawliet_types::{
    actor::ActorDisplay,
    channel::{AlivePolicy, ChannelKind, PermUpdatePolicy},
    command::Command,
};

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, AddState, CreateAndGiveProfile, CreateChannel, ReleaseKidnapping,
        ScheduleJob,
    },
    actor::{modifier::Modifier, state::State},
    common::{KidnappingKey, Version},
    engine::Engine,
    helpers::{cmd_channel, cmd_world_event, get_ability, get_actor, require_player},
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

        let channel_response = Action::CreateChannel(CreateChannel {
            loggable: true,
            base_profile: None,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;
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
                mask: None,
            })
        } else {
            KidnappingKey::default()
        };

        cmd_channel(
            eng,
            ctx,
            Command::MapChannel {
                channel_id,
                kind: ChannelKind::Kidnapping(id),
            },
            channel_id,
            false,
            None,
        );

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

        // The victim, and only the victim. They talk as themselves for as long as they are alive
        // to; being held is what put them here, so nothing about being held is asked again.
        //
        // The kidnapper side cannot be settled here. It is whoever the source ability belongs to,
        // which for an org is a roster that changes underneath the kidnapping — people join and
        // leave while somebody is still being held — so it has to be re-derived rather than seated
        // once.
        //
        // Gated, like everything downstream of a channel this action itself created: on the
        // validate pass there is no channel yet to put a name in.
        if mutate {
            Action::CreateAndGiveProfile(CreateAndGiveProfile {
                channel_id,
                player_id: self.victim_id,
                display: ActorDisplay::Raw(self.victim_id),
                visible: true,
                shared: false,
                transferrable: false,
                perm_policy: PermUpdatePolicy::Alive(AlivePolicy {}),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::CreateKidnapping(CreateKidnappingResponse {
            id,
        }))
    }
}
