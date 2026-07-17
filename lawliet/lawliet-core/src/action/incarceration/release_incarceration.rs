/*
* ADMIN / SYSTEM / ABILITY-OWNER ACTION
* Release an incarcerated player.
*
* Authorization: actor must be authoritative, or own the incarceration's source ability.
*
* On execution:
* - remove incarceration record (before RemoveState so UpdateIncarcerationChannels sees it gone)
* - RemoveState(victim, State::Incarcerated)
*
* TODO: commands
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, RemoveState,
    },
    actor::state::State,
    common::Version,
    engine::Engine,
    helpers::{actor_owns_ability, get_incarceration},
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

        Ok(ActionResponse::ReleaseIncarceration(
            ReleaseIncarcerationResponse {},
        ))
    }
}
