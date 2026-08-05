/*
* SYSTEM ACTION
* Give an ability to an org including the org ability metadata
*/

use lawliet_types::command::Command;

use crate::{
    action::{Action, ActionInterface, ActionResponse, GiveAbility},
    helpers::{get_org, get_org_mut, owner_view_recipient},
};

// TODO:
// new action for modifying owned ability metadata

use crate::action::ActionActor;
pub use crate::action::{GiveOrgAbility, GiveOrgAbilityResponse};

impl ActionInterface for GiveOrgAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        get_org(eng, self.org_id)?;

        Action::GiveAbility(GiveAbility {
            ability_id: self.ability_id,
            actor_id: self.org_id,
            volatile: false,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        let org = get_org_mut(eng, self.org_id)?;
        if mutate {
            org.add_ability(self.ability_id, self.settings.clone());

            // The ability view has just been emitted to the org's viewport by GiveAbility above;
            // its static gates ride the same viewport right after so the menu can state them, and
            // so a member entering later replays both together.
            ctx.push_cmd(
                Command::OrgAbilityRequirements {
                    ability_id: self.ability_id,
                    requirements: self.settings.clone(),
                },
                owner_view_recipient(eng, self.org_id),
                eng.time,
            );
        }

        Ok(ActionResponse::GiveOrgAbility(GiveOrgAbilityResponse {}))
    }
}
