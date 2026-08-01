use lawliet_types::{
    action::{
        Action, ActionActor, ActionError, ActionResponse, CreateAndGiveProfile, CreateChannel,
        CreatePersonalChannel, CreatePersonalChannelResponse,
    },
    actor::ActorDisplay,
    channel::{ChannelPerm, FixedPolicy, PermUpdatePolicy},
};

use crate::{
    action::ActionInterface,
    channel::ChannelKind,
    command::Command,
    helpers::{actor_id, cmd_channel, get_player, get_player_mut},
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

        let channel_response = Action::CreateChannel(CreateChannel {
            loggable: false,
            base_profile: None,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        let ActionResponse::CreateChannel(data) = channel_response else {
            unreachable!();
        };
        let channel_id = data.id;

        // Addressed to the channel's own viewport, so only the owner ever sees it, and pushed
        // before the name that puts them in it — the channel has to exist before anything can be
        // said about a name in it.
        cmd_channel(
            eng,
            ctx,
            Command::MapChannel {
                channel_id,
                kind: ChannelKind::Personal,
            },
            channel_id,
            false,
            None,
        );

        if mutate {
            let player = get_player_mut(eng, player_id).expect("already validated");
            player.personal_channels.insert(channel_id);
            player.personal_channel_charges = player.personal_channel_charges.saturating_sub(1);

            // Their own notepad, and theirs unconditionally: nothing that happens to them takes
            // away a place that is only ever read by them.
            Action::CreateAndGiveProfile(CreateAndGiveProfile {
                channel_id,
                player_id,
                display: ActorDisplay::Raw(player_id),
                visible: true,
                shared: false,
                transferrable: false,
                perm_policy: PermUpdatePolicy::Fixed(FixedPolicy {
                    perms: ChannelPerm::Send | ChannelPerm::View | ChannelPerm::LoggabilityControl,
                }),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::CreatePersonalChannel(
            CreatePersonalChannelResponse { id: channel_id },
        ))
    }
}
