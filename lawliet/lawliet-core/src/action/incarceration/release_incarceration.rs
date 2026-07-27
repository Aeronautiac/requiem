/*
* ADMIN / SYSTEM / ABILITY-OWNER ACTION
* Release an incarcerated player.
*
* Authorization: actor must be authoritative, or own the incarceration's source ability.
*
* On execution:
* - remove incarceration record (before RemoveState so UpdateIncarcerationChannels sees it gone)
* - RemoveState(victim, State::Incarcerated)
* - announce the release as a world event
*
* The announcement carries only the id. Who ordered the incarceration is never disclosed, so there
* is nothing here corresponding to KidnapReveal.
*/

use lawliet_types::command::Command;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, RemoveState,
    },
    actor::state::State,
    common::Version,
    engine::Engine,
    helpers::{actor_owns_ability, cmd_world_event, get_incarceration},
    incarceration::IncarcerationSource,
};

pub use crate::action::{ReleaseIncarceration, ReleaseIncarcerationResponse};

impl ActionInterface for ReleaseIncarceration {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        let incarceration = get_incarceration(eng, self.incarceration_id)?;
        let victim_id = incarceration.victim;

        let authorized = actor.is_authoritative()
            || matches!(incarceration.source, IncarcerationSource::Ability(ab) if actor_owns_ability(eng, actor, ab));
        if !authorized {
            return Err(ActionError::InsufficientPermissions);
        }

        if mutate {
            eng.world.remove_incarceration(self.incarceration_id);
        }

        Action::RemoveState(RemoveState {
            actor_id: victim_id,
            state: State::Incarcerated,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        // After RemoveState, which is what returns their presence — announced to the viewport they
        // have just rejoined, so the released player sees their own release.
        cmd_world_event(
            eng,
            ctx,
            Command::IncarcerationReleased {
                incarceration_id: self.incarceration_id,
            },
        );

        Ok(ActionResponse::ReleaseIncarceration(
            ReleaseIncarcerationResponse {},
        ))
    }
}
