/*
* Authoritative Action
* Drop any press-conference guest who can no longer be there.
*
* Being in the conference is what grants a guest news Send (see NewsPolicy), and the only thing
* keeping the roster honest as the game moves. Eligibility mirrors the check PressConfAccess makes on
* the way in: a guest who has lost presence (dead, kidnapped, jailed) is removed here, and the same
* PressConfStatus that announced their entry announces their exit. Runs in the Update chain before
* the channel sweep, so the news perms recompute already reflects the removal.
*/

use lawliet_types::{
    actor::Modifier,
    command::{Command, CommandRecipient},
};
use smallvec::SmallVec;

use crate::{
    ActorKey,
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    helpers::get_actor,
};

pub use crate::action::{UpdatePressConference, UpdatePressConferenceResponse};

impl ActionInterface for UpdatePressConference {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Gone if the actor no longer exists, or is present in name only.
        let to_remove: SmallVec<[ActorKey; 8]> = eng
            .world
            .news
            .press_conf
            .iter()
            .copied()
            .filter(|id| {
                get_actor(eng, *id).map_or(true, |a| a.has_modifier(Modifier::NoPresence))
            })
            .collect();

        for id in to_remove {
            if mutate {
                eng.world.news.press_conf.swap_remove(&id);
            }
            ctx.push_cmd(
                Command::PressConfStatus {
                    target_id: id,
                    has_access: false,
                },
                CommandRecipient::Viewport(eng.world.events_viewport),
                eng.time,
            );
        }

        Ok(ActionResponse::UpdatePressConference(
            UpdatePressConferenceResponse {},
        ))
    }
}
