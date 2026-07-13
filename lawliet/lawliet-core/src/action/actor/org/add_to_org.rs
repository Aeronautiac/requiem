/*
* SYSTEM ACTION
* Add a player to an organization
*/

use indexmap::indexset;
use lawliet_types::{
    action::{SetMember, UpdateContactChannels},
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermissions},
    command::{Command, CommandRecipient},
};

use crate::{
    action::{
        Action, ActionError, ActionInterface, ActionResponse, ChangeOrgLeader, UpdateKidnapChannels,
    },
    actor::{ActorLink, ActorLinkType},
    helpers::{get_actor_mut, get_org_mut, get_player, get_player_mut},
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

        // Surface the org membership. The org member list is the same for everyone right
        // now, so this is undirected (System); the frontend keys it by org_id. This is the
        // org member list, distinct from the org channel's member list (SetMember below).
        ctx.push_cmd(
            Command::AddOrgMember {
                player_id: self.actor_id,
                org_id: self.org_id,
            },
            CommandRecipient::System,
            eng.time,
        );

        Action::SetMember(SetMember {
            player_id: self.actor_id,
            settings: Some(ChannelMember {
                perms: ChannelPermissions::EMPTY,
                displays: indexset! { ActorDisplay::Raw(self.actor_id) },
            }),
            channel_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdateContactChannels(UpdateContactChannels {
            player_id: self.actor_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        // TODO:
        // Notify member of leadership change and membership

        Action::UpdateKidnapChannels(UpdateKidnapChannels {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::AddToOrg(AddToOrgResponse {}))
    }
}
