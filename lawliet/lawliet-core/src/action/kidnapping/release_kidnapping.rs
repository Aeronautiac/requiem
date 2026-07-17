/*
* ADMIN / SYSTEM / ABILITY-OWNER ACTION
* Release a kidnapped player.
*
* Authorization: actor must be authoritative, or own the kidnapping's source ability.
*
* On execution:
* - remove kidnapping record (before RemoveState so UpdateKidnapChannels sees it gone)
* - RemoveState(victim, State::Kidnapped)
* - DestroyChannel(channel, archive: true)
*
* TODO: commands (reveal kidnapper identity if public kidnapping)
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, DestroyChannel, RemoveState,
    },
    actor::{ActorDisplay, modifier::Modifier, state::State},
    command::Command,
    common::Version,
    engine::Engine,
    helpers::{actor_owns_ability, cmd_all_deferred, get_kidnapping},
    kidnapping::{KidnappingSource, KidnappingType},
};

pub use crate::action::{ReleaseKidnapping, ReleaseKidnappingResponse};

impl ActionInterface for ReleaseKidnapping {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        let kidnapping = get_kidnapping(eng, self.kidnapping_id)?;
        let victim_id = kidnapping.victim;
        let channel_id = kidnapping.channel_id;
        // Who to reveal: a public kidnapping leaks the kidnapper (the display's raw actor);
        // an anonymous one reveals no one. Captured before the record is removed below.
        let kidnapper = match kidnapping.kidnapping_type {
            KidnappingType::Public(ActorDisplay::Raw(id)) => Some(id),
            _ => None,
        };

        let authorized = actor.is_authoritative()
            || matches!(kidnapping.source, KidnappingSource::Ability(ab) if actor_owns_ability(eng, actor, ab));
        if !authorized {
            return Err(ActionError::InsufficientPermissions);
        }

        if mutate {
            eng.world.remove_kidnapping(self.kidnapping_id);
        }

        Action::DestroyChannel(DestroyChannel {
            channel_id,
            archive: !self.forced,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Action::RemoveState(RemoveState {
            actor_id: victim_id,
            state: State::Kidnapped,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        // Announce the reveal (references the kidnapping by id so clients resolve the victim).
        cmd_all_deferred(
            eng,
            ctx,
            Command::KidnapReveal {
                kidnapping_id: self.kidnapping_id,
                kidnapper,
            },
            Modifier::NoPresence.into(),
            true,
            true,
            mutate,
        );

        Ok(ActionResponse::ReleaseKidnapping(
            ReleaseKidnappingResponse {},
        ))
    }
}
