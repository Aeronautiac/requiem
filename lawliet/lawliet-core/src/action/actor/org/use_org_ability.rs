/*
* PLAYER ONLY
* Try to use an organization ability
* This action wraps SystemUseOrgAbility
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        SystemUseOrgAbility,
    },
    helpers::{actor_id, require_running},
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

        let response = Action::SystemUseOrgAbility(SystemUseOrgAbility {
            org_id: self.org_id,
            user_id: actor_id(actor).unwrap(),
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
