/*
* PLAYER ONLY
* Try to use an organization ability
* This action wraps SystemUseOrgAbility
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, SystemUseOrgAbility,
    },
    helpers::{actor_id, get_org, require_running},
};

pub use crate::action::{UseOrgAbility, UseOrgAbilityResponse};

impl ActionInterface for UseOrgAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.player_only()?;
        // Also gated here, not just in UseAbility: an org ability may open a VOTE rather than fire,
        // and a vote opened during setup would sit there waiting to resolve into a use that cannot
        // happen.
        require_running(eng)?;

        // An org's abilities belong to the org, and only its members may reach for one. This is the
        // ONLY place that is checked, and it has to be here rather than in the primitive below:
        // ownership passes for anyone who names the ability (the owner is the ORG), and the gates
        // the primitive does apply — presence, member count, required roles — are all about the
        // org's condition rather than about who is asking.
        //
        // SystemUseOrgAbility deliberately keeps no equivalent. It is what an admin drives and what
        // a resolved vote fires through, and a vote that passed must still execute even if the
        // member who opened it has since left.
        let user_id = actor_id(actor).unwrap();
        if !get_org(eng, self.org_id)?.has_member(user_id) {
            return Err(ActionError::PlayerNotInOrg);
        }

        let response = Action::SystemUseOrgAbility(SystemUseOrgAbility {
            org_id: self.org_id,
            user_id,
            ability_id: self.ability_id,
            ability_args: self.ability_args.clone(),
            dont_vote: false,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        let ActionResponse::SystemUseOrgAbility(use_response) = response else {
            unreachable!()
        };
        let poll_id = use_response.poll_id;

        Ok(ActionResponse::UseOrgAbility(UseOrgAbilityResponse {
            poll_id,
        }))
    }
}
