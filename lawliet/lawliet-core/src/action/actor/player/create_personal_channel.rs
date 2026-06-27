use indexmap::indexset;
use lawliet_types::{
    action::{
        Action, ActionError, ActionResponse, CreateChannel, CreatePersonalChannel,
        CreatePersonalChannelResponse, SetMember,
    },
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermission},
};

use crate::{
    action::ActionInterface,
    helpers::{actor_id, get_player, get_player_mut},
};

impl ActionInterface for CreatePersonalChannel {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        version: lawliet_types::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_only()?;
        let player_id = actor_id(actor).expect("already validated as a player");
        let player = get_player(eng, player_id).expect("player should already be validated");

        if player.personal_channel_charges == 0 {
            return Err(ActionError::PersonalChannelLimitReached);
        }

        let channel_response = Action::CreateChannel(CreateChannel { loggable: true })
            .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::CreateChannel(data) = channel_response else {
            unreachable!();
        };
        let channel_id = data.id;

        if mutate {
            let player = get_player_mut(eng, player_id).expect("already validated");
            player.personal_channels.insert(channel_id);
            player.personal_channel_charges -= 1;

            Action::SetMember(SetMember {
                player_id,
                channel_id,
                settings: Some(ChannelMember {
                    perms: ChannelPermission::Send
                        | ChannelPermission::View
                        | ChannelPermission::LoggabilityControl,
                    displays: indexset! { ActorDisplay::Raw(player_id) },
                }),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::CreatePersonalChannel(
            CreatePersonalChannelResponse { id: channel_id },
        ))
    }
}
