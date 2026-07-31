/*
* Player and System Action
* Set the loggable status of a channel
*/

use lawliet_types::{action::ActionError, channel::ChannelPerm};

use crate::{
    action::{ActionInterface, ActionResponse},
    command::Command,
    helpers::{cmd_channel, get_channel_mut, player_id},
};

use crate::action::ActionActor;
pub use crate::action::{SetLoggable, SetLoggableResponse};

impl ActionInterface for SetLoggable {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_or_authoritative()?;

        // Only allow it under a name that carries the permission. Any of theirs will do: holding
        // two names here means being able to do whatever either of them can.
        let channel = get_channel_mut(eng, self.channel_id)?;
        if actor.is_player() {
            let id = player_id(actor).expect("already validated as a player");
            let permitted = channel
                .owned_profiles(id)
                .any(|profile| profile.perms.contains(ChannelPerm::LoggabilityControl));
            if !permitted {
                return Err(ActionError::InsufficientPermissions);
            }
        }

        if mutate {
            channel.loggable = self.loggable
        }

        // Addressed to the channel itself, like the initial value emitted from CreateChannel,
        // so every viewer's channel UI reflects it.
        cmd_channel(
            eng,
            ctx,
            Command::SetChannelLoggable {
                channel_id: self.channel_id,
                loggable: self.loggable,
            },
            self.channel_id,
            false,
            None,
        );

        Ok(ActionResponse::SetLoggable(SetLoggableResponse {}))
    }
}
