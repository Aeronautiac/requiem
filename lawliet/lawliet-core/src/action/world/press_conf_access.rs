// news anchor is a weird kind of role because it can potentially shift around as a possession rather than
// being treated as a static role depending on what the host wants.
//
// the plan:
// be aware of this limitation, but it doesn't really matter much right now.
// news anchor is a role in the current version of this game.
// a role rework could be done in the future where roles become stateful entities, and you
// could for instance own multiple roles, which each own multiple items, etc...
// this quickly devolves into a state nightmare to which the solution is ecs, but the engine is not
// built around that, and it doesnt need to be.
//
// a simple fix right now:
// remove the news anchor role entirely, and just have a set of ability ids held within a struct on
// the world, and dynamically change the ability owners to whoever the current news anchor is. this
// is two actions (init + news anchor status), and requires zero restructuring as abilities can
// already exist without an owner.

use crate::{
    action::ActionInterface,
    helpers::{actor_get_effective_passive, actor_id, get_actor, get_player},
};
use lawliet_types::{
    action::{ActionError, ActionResponse, PressConfAccess, PressConfAccessResponse},
    actor::Modifier,
    command::{Command, CommandRecipient},
    passive::PassiveType,
};

impl ActionInterface for PressConfAccess {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        _version: lawliet_types::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        get_player(eng, self.target_id)?;

        if !actor.is_authoritative() {
            let Some(id) = actor_id(actor) else {
                unreachable!()
            };

            let user_data = get_actor(eng, id).expect("already validated actor id");
            if user_data.has_modifier(Modifier::NoPresence) {
                return Err(ActionError::UserNotPresent);
            }

            if actor_get_effective_passive(eng, id, |p| matches!(p, PassiveType::NewsControl))
                .is_none()
            {
                return Err(ActionError::NoNewsControl);
            }
        }

        let target_data = get_actor(eng, self.target_id).expect("already validated");
        if self.has_access {
            if target_data.has_modifier(Modifier::NoPresence) {
                return Err(ActionError::UserNotPresent);
            }
            if eng.world.news.press_conf.len() >= eng.config.defaults.press_conf_limit as usize {
                return Err(ActionError::ConferenceFull);
            }

            if mutate {
                eng.world.news.press_conf.insert(self.target_id);
            }
        } else {
            if !eng.world.news.press_conf.contains(&self.target_id) {
                return Err(ActionError::NotInConference);
            }

            if mutate {
                eng.world.news.press_conf.swap_remove(&self.target_id);
            }
        }

        ctx.push_cmd(
            Command::PressConfStatus {
                target_id: self.target_id,
                has_access: self.has_access,
            },
            CommandRecipient::Viewport(eng.world.events_viewport),
            eng.time,
        );

        Ok(ActionResponse::PressConfAccess(PressConfAccessResponse {}))
    }
}
