/*
* SYSTEM ONLY
* Try to use an organization ability
*/

use indexmap::IndexSet;

use crate::{
    ability::AbilityBehaviour,
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionError, ActionResponse, OrgActorInfo, UseAbility, CreatePoll,
    },
    actor::{modifier::Modifier, organization::OrgAbilityPolicy},
    common::{AbilityKey, ActorKey, PollKey},
    config::role::Role,
    helpers::{get_actor, get_org},
    poll::{PollPolicy, PollVisibility, VoterPolicy},
};

pub use crate::action::{SystemUseOrgAbility, SystemUseOrgAbilityResponse};

impl ActionInterface for SystemUseOrgAbility {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let mut id = None;
        if let Ok(org_data) = get_org(eng, self.org_id) {
            let player_id = self.user_id;
            let player_data = get_actor(eng, player_id)?;
            if player_data.has_modifier(Modifier::NoPresence) {
                return Err(ActionError::UserNotPresent);
            }

            let org_ability = org_data.abilities.get(&self.ability_id).unwrap();
            if org_data.member_count(|id, _| {
                let member_data = eng.world.get_actor(id).unwrap();
                !member_data.has_modifier(Modifier::NoPresence)
            }) < org_ability.require_members
            {
                return Err(ActionError::NotEnoughMembers);
            }

            let available_roles: IndexSet<Role> = org_data
                .members
                .keys()
                .filter(|id| {
                    let actor_data = eng.world.get_actor(**id).unwrap();
                    !actor_data.has_modifier(Modifier::NoPresence)
                })
                .map(|id| eng.world.get_player(*id).unwrap().role)
                .collect();
            for role in &org_ability.require_roles {
                if !available_roles.contains(role) {
                    return Err(ActionError::RequiredRolesNotPresent);
                }
            }

            let ability_policies = org_ability.usage_policies;
            if ability_policies.contains(OrgAbilityPolicy::RequireLeader) {
                let Some(leadership_struct) = &org_data.leadership_struct else {
                    unreachable!(); // there must be a leadership struct if the ability requires a leader
                };
                if leadership_struct.leader != Some(player_id) {
                    return Err(ActionError::PlayerIsNotLeader);
                }
            }

            if !self.dont_vote && ability_policies.contains(OrgAbilityPolicy::RequireVote) {
                let response = Action::CreatePoll(CreatePoll {
                    voter_policy: VoterPolicy::Present,
                    visibility: PollVisibility::Org(self.org_id),
                    update_policy: PollPolicy::Majority,
                    timeout_policy: PollPolicy::Majority,
                    accept_payload: Box::new(Some(Action::SystemUseOrgAbility(
                        SystemUseOrgAbility {
                            org_id: self.org_id,
                            user_id: self.user_id,
                            ability_id: self.ability_id,
                            ability_args: self.ability_args.clone(),
                            dont_vote: true,
                        },
                    ))),
                    reject_payload: Box::new(None),
                    duration: Some(eng.config.defaults.org_vote_time),
                })
                .handle(eng, ctx, &ActionActor::System, version, mutate)?;
                let ActionResponse::CreatePoll(create_poll_response) = response else {
                    unreachable!();
                };
                id = Some(create_poll_response.id);
            } else {
                Action::UseAbility(UseAbility {
                    ability_id: self.ability_id,
                    ability_args: self.ability_args.clone(),
                })
                .handle(
                    eng,
                    ctx,
                    &ActionActor::Organization(OrgActorInfo {
                        player_id,
                        org_id: self.org_id,
                    }),
                    version,
                    mutate,
                )?;
            }
        }

        Ok(ActionResponse::SystemUseOrgAbility(
            SystemUseOrgAbilityResponse { poll_id: id },
        ))
    }
}
