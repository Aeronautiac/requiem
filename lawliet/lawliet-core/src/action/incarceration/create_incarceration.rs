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
* - schedule the release when `duration` is Some
* - announce it as a world event
*
* The announcement names the victim and never the source: an incarceration's source is not
* disclosed, and no later reveal changes that (unlike a kidnapping's).
*
* TODO: potentially move the strengthened presence check(s) to the ability level rather than the
* system level
*/

use lawliet_types::command::Command;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, AddState, ReleaseIncarceration, ScheduleJob,
    },
    actor::{modifier::Modifier, state::State},
    common::{IncarcerationKey, Version},
    engine::Engine,
    helpers::{cmd_world_event, get_ability, get_actor, require_player},
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

        let id = if mutate {
            eng.world.add_incarceration(Incarceration {
                victim: self.victim_id,
                source: self.source,
            })
        } else {
            IncarcerationKey::default()
        };

        // Announced BEFORE the state change. Incarcerated carries NoPresence, which takes the
        // victim out of the very viewport this is addressed to — announcing afterwards tells
        // everyone except the person it happened to, who is left guessing why they went dark.
        cmd_world_event(
            eng,
            ctx,
            Command::Incarceration {
                incarceration_id: id,
                victim_id: self.victim_id,
                duration: self.duration,
            },
        );

        Action::AddState(AddState {
            actor_id: self.victim_id,
            state: State::Incarcerated,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        if let Some(duration) = self.duration {
            Action::ScheduleJob(ScheduleJob {
                payload: Box::new(Action::ReleaseIncarceration(ReleaseIncarceration {
                    incarceration_id: id,
                    forced: false,
                })),
                timestamp: eng.time + duration,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::CreateIncarceration(
            CreateIncarcerationResponse { id },
        ))
    }
}
