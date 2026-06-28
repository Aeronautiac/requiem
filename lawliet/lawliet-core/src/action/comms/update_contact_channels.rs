/*
* SYSTEM ACTION
* Update a player's contact channel (groupchats, lounges, and orgs) permissions based on current state.
*/

use crate::{
    action::{Action, ActionInterface, ActionResponse, SetMember},
    actor::modifier::Modifier,
    channel::{ChannelPermission, ChannelPermissions},
    helpers::{get_actor, get_channel, get_channel_mut, get_gc, get_lounge, get_org, get_player},
};

use crate::action::ActionActor;
pub use crate::action::{UpdateContactChannels, UpdateContactChannelsResponse};

impl ActionInterface for UpdateContactChannels {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let actor_data = get_actor(eng, self.player_id)?;
        let no_contact = actor_data.has_modifier(Modifier::NoContact);

        let player_data = get_player(eng, self.player_id)?;
        let lounges = player_data.lounges.clone();
        let gcs = player_data.groupchats.clone();
        let orgs = player_data.orgs.clone();

        for org_id in orgs {
            let org = get_org(eng, org_id).expect("expected a valid org within the player's cache");
            let channel_id = org.channel_id;
            let channel = get_channel(eng, channel_id)?;
            let mut member_settings = channel
                .get_member(self.player_id)
                .expect("expected player to be in an org channel within their lounge cache")
                .clone();
            if no_contact {
                member_settings.perms = ChannelPermissions::EMPTY;
            } else {
                member_settings.perms = ChannelPermission::Send | ChannelPermission::View;
            }
            Action::SetMember(SetMember {
                player_id: self.player_id,
                channel_id,
                settings: Some(member_settings),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        for lounge_id in lounges {
            let lounge = get_lounge(eng, lounge_id)?;
            let channel_id = lounge.channel_id;
            let channel = get_channel_mut(eng, lounge.channel_id)?;
            let mut member_settings = channel
                .get_member(self.player_id)
                .expect("expected player to be in a lounge within their lounge cache")
                .clone();
            if no_contact {
                member_settings.perms = ChannelPermissions::EMPTY;
            } else {
                member_settings.perms = ChannelPermission::Send | ChannelPermission::View;
            }
            Action::SetMember(SetMember {
                player_id: self.player_id,
                channel_id,
                settings: Some(member_settings),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        for gc_id in gcs {
            let gc = get_gc(eng, gc_id)?;
            let channel_id = gc.channel_id;
            let channel = get_channel_mut(eng, gc.channel_id)?;
            let mut member_settings = channel
                .get_member(self.player_id)
                .expect("expected player to be in a gc within their gc cache")
                .clone();
            if no_contact {
                member_settings.perms = ChannelPermissions::EMPTY;
            } else {
                member_settings.perms = ChannelPermission::Send | ChannelPermission::View;
            }
            Action::SetMember(SetMember {
                player_id: self.player_id,
                channel_id,
                settings: Some(member_settings),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::UpdateContactChannels(
            UpdateContactChannelsResponse {},
        ))
    }
}
