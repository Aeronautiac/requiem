use indexmap::indexset;
use lawliet_types::{
    action::{
        Action, ActionActor, ActionError, ActionResponse, CreateChannel, CreatePersonalChannel,
        CreatePersonalChannelResponse, SetMember,
    },
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermission},
    command::CommandRecipient,
};

use crate::{
    action::ActionInterface,
    command::Command,
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

        let channel_response = Action::CreateChannel(CreateChannel { loggable: false }).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;
        let ActionResponse::CreateChannel(data) = channel_response else {
            unreachable!();
        };
        let channel_id = data.id;

        // Tag the freshly-created channel as a personal channel on the frontend. Must precede
        // the SetMember below (whose UpdateChannelView references the channel), and mirrors how
        // the other channel kinds announce themselves (MapGc, MapLounge, …). Global, like them.
        ctx.push_cmd(
            Command::MapPersonalChannel { channel_id },
            CommandRecipient::System,
            eng.time,
        );

        if mutate {
            let player = get_player_mut(eng, player_id).expect("already validated");
            player.personal_channels.insert(channel_id);
            player.personal_channel_charges = player.personal_channel_charges.saturating_sub(1);

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
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::CreatePersonalChannel(
            CreatePersonalChannelResponse { id: channel_id },
        ))
    }
}
