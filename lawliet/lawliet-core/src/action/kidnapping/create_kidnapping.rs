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

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionError, ActionResponse, AddState, CreateChannel, UpdateKidnapChannels,
    },
    actor::modifier::Modifier,
    actor::state::State,
    common::{ActorKey, KidnappingKey, Version},
    engine::Engine,
    helpers::{get_ability, get_actor, require_player},
    kidnapping::{Kidnapping, KidnappingSource, KidnappingType},
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

        Action::AddState(AddState {
            actor_id: self.victim_id,
            state: State::Kidnapped,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

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

        Action::UpdateKidnapChannels(UpdateKidnapChannels {})
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(ActionResponse::CreateKidnapping(CreateKidnappingResponse {
            id,
        }))
    }
}
