/*
* SYSTEM ACTION
* Create a bug that relays messages from a target player to a bug log
*/

use crate::{
    action::{
        ActionInterface, Action, ActionResponse, UpdateBugVisibilities,
    },
    bug::{Bug, BugSource},
    command::Command,
    common::{ActorKey, BugKey},
    helpers::{get_ability, get_player_mut},
};

use crate::action::ActionActor;
pub use crate::action::{CreateBug, CreateBugResponse};

impl ActionInterface for CreateBug {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        get_player_mut(eng, self.target_id)?;

        if let BugSource::Ability(ability_id) = self.source {
            get_ability(eng, ability_id)?;
        }

        let id = if mutate {
            let bug_id = eng.world.add_bug(Bug::new(self.target_id, self.source));
            get_player_mut(eng, self.target_id)
                .expect("expected valid target player")
                .add_bug(bug_id);

            // this needs to come before the visibility update action
            ctx.push_cmd(Command::NewBug { bug_key: bug_id }, None, eng.time);

            bug_id
        } else {
            BugKey::default()
        };

        Action::UpdateBugVisibilities(UpdateBugVisibilities {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::CreateBug(CreateBugResponse { id }))
    }
}
