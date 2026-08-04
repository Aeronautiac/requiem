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
        require_running(eng)?;

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
