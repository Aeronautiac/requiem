/*
* SYSTEM ACTION
* Add a player to an organization
*/

use lawliet_types::{
    action::CreateAndGiveProfile,
    actor::ActorDisplay,
    channel::{ContactPolicy, PermUpdatePolicy},
    command::Command,
};

use crate::{
    action::{Action, ActionError, ActionInterface, ActionResponse, ChangeOrgLeader},
    actor::{ActorLink, ActorLinkType},
    helpers::{
        cmd_channel, cmd_og_status, get_actor_mut, get_org, get_org_mut, get_player, get_player_mut,
    },
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
        get_player(eng, self.actor_id)?;

        let org = get_org_mut(eng, self.org_id)?;
        let channel_id = org.channel_id;
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

            // not possible to already be the leader because they cant have already been in the org
            if self.leader {
                Action::ChangeOrgLeader(ChangeOrgLeader {
                    org_id: self.org_id,
                    new_leader: Some(self.actor_id),
                })
                .handle(eng, ctx, actor, version, mutate)?;
            }

            let player_data = get_player_mut(eng, self.actor_id).expect("already validated player");
            player_data.orgs.insert(self.org_id);
        }

        // Their OG standing, which joining is the first thing to decide. Personal, so it goes to
        // them and to System rather than onto the roster below — the org learns that they joined,
        // not what they joined as.
        cmd_og_status(eng, ctx, self.org_id, self.actor_id, self.og);

        // Surface the org membership, addressed to the org's backing channel: who may see an
        // org's roster is exactly who may see the org's channel. This is the org member list,
        // distinct from the org channel's member list (SetMember below).
        let org_channel = get_org(eng, self.org_id)?.channel_id;
        cmd_channel(
            eng,
            ctx,
            Command::AddOrgMember {
                player_id: self.actor_id,
                org_id: self.org_id,
            },
            org_channel,
            false,
            None,
        );

        Action::CreateAndGiveProfile(CreateAndGiveProfile {
            channel_id,
            player_id: self.actor_id,
            display: ActorDisplay::Raw(self.actor_id),
            visible: true,
            shared: false,
            transferrable: false,
            perm_policy: PermUpdatePolicy::Contact(ContactPolicy {}),
        })
        .handle(eng, ctx, actor, version, mutate)?;

        // TODO:
        // Notify member of leadership change and membership

        Ok(ActionResponse::AddToOrg(AddToOrgResponse {}))
    }
}
