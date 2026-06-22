/*
* SYSTEM / ADMIN ACTION
* Incarcerate a player: apply State::Incarcerated and add them to the Prison world channel.
*
* Preconditions:
* - victim exists, is a player, does not have NoPresence, does not have StrengthenedPresence
* - if source is Ability, that ability must exist
*
* On execution:
* - AddState(victim, State::Incarcerated)
* - store Incarceration in world
* - UpdateIncarcerationChannels (grants victim Send | View on Prison channel)
*
* TODO: commands
* also potentially move the strengthened presence check(s) to the ability level rather than the system
* level
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionError, ActionResponse, AddState,
    },
    actor::{modifier::Modifier, state::State},
    common::{ActorKey, IncarcerationKey, Version},
    engine::Engine,
    helpers::{get_ability, get_actor, require_player},
    incarceration::{Incarceration, IncarcerationSource},
};

pub use crate::action::{CreateIncarceration, CreateIncarcerationResponse};

impl ActionInterface for CreateIncarceration {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        if let IncarcerationSource::Ability(ab) = self.source {
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

        Action::AddState(AddState {
            actor_id: self.victim_id,
            state: State::Incarcerated,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        let id = if mutate {
            eng.world.add_incarceration(Incarceration {
                victim: self.victim_id,
                source: self.source,
            })
        } else {
            IncarcerationKey::default()
        };

        Ok(ActionResponse::CreateIncarceration(
            CreateIncarcerationResponse { id },
        ))
    }
}
