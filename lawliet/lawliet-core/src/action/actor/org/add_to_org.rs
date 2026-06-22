/*
* SYSTEM ACTION
* Add a player to an organization
*/

use crate::{
    action::{
        ActionInterface, Action, ActionError, ActionResponse, ChangeOrgLeader, UpdateKidnapChannels,
    },
    actor::{ActorLink, ActorLinkType},
    common::ActorKey,
    helpers::{get_actor, get_actor_mut, get_org_mut},
};

use crate::action::ActionActor;
pub use crate::action::{AddToOrg, AddToOrgResponse};

impl ActionInterface for AddToOrg {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        get_actor(eng, self.actor_id)?;

        let org = get_org_mut(eng, self.org_id)?;
        if org.has_member(self.actor_id) {
            return Err(ActionError::ActorAlreadyInOrg);
        }
        if org.leadership_struct.is_none() && self.leader {
            return Err(ActionError::OrgDoesntHaveLeadership);
        }
        if org.is_blacklisted(self.actor_id) {
            return Err(ActionError::PlayerIsBlacklisted);
        }

        // keep in mind that the leader is replaced if leader is true. the case where there was a
        // previous leader should be handled (notify them that they have lost leadership).
        if mutate {
            org.add_member(self.actor_id, self.og);
            let actor_data = get_actor_mut(eng, self.actor_id)?;
            actor_data.add_link(ActorLink {
                link_type: ActorLinkType::Passive,
                link_dest: self.org_id,
            });

            // TODO:
            // Notify member of leadership change and membership

            // not possible to already be the leader because they cant have already been in the org
            if self.leader {
                Action::ChangeOrgLeader(ChangeOrgLeader {
                    org_id: self.org_id,
                    new_leader: Some(self.actor_id),
                })
                .handle(eng, ctx, actor, version, mutate)?;
            }
        }

        Action::UpdateKidnapChannels(UpdateKidnapChannels {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::AddToOrg(AddToOrgResponse {}))
    }
}
