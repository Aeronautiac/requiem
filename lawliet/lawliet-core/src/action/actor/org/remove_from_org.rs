/*
* SYSTEM ACTION
* Remove a player from an organization
*/

use lawliet_types::{
    action::SetMember,
    command::{Command, CommandRecipient},
};

use crate::{
    action::{Action, ActionError, ActionInterface, ActionResponse, UpdateKidnapChannels},
    actor::{ActorLink, ActorLinkType},
    helpers::{get_actor_mut, get_org_mut, get_player, get_player_mut},
};

use crate::action::ActionActor;
pub use crate::action::{RemoveFromOrg, RemoveFromOrgResponse};

impl ActionInterface for RemoveFromOrg {
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
        if !org.has_member(self.actor_id) {
            return Err(ActionError::PlayerNotInOrg);
        }

        if mutate {
            org.remove_member(self.actor_id);
            let actor = get_actor_mut(eng, self.actor_id)?;
            actor.sever_link(ActorLink {
                link_type: ActorLinkType::Passive,
                link_dest: self.org_id,
            });

            let player = get_player_mut(eng, self.actor_id).expect("should already be validated");
            player.orgs.swap_remove(&self.org_id);
        }

        // Undirected (System): the org member list is the same for everyone right now.
        ctx.push_cmd(
            Command::RemoveOrgMember {
                player_id: self.actor_id,
                org_id: self.org_id,
            },
            CommandRecipient::System,
            eng.time,
        );

        Action::SetMember(SetMember {
            player_id: self.actor_id,
            settings: None,
            channel_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdateKidnapChannels(UpdateKidnapChannels {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::RemoveFromOrg(RemoveFromOrgResponse {}))
    }
}
