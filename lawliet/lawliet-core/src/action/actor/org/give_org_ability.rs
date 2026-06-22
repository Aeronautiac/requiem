/*
* SYSTEM ACTION
* Give an ability to an org including the org ability metadata
*/

use crate::{
    action::{
        ActionInterface, Action, ActionResponse, GiveAbility,
    },
    actor::organization::OrgAbility,
    common::{AbilityKey, ActorKey},
    helpers::{get_org, get_org_mut},
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
        }

        Ok(ActionResponse::GiveOrgAbility(GiveOrgAbilityResponse {}))
    }
}
