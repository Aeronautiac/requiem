/*
* Player and System Action
* Set the loggable status of a channel
*/

use lawliet_types::{action::ActionError, channel::ChannelPermission, command::CommandRecipient};

use crate::{
    action::{ActionInterface, ActionResponse},
    command::Command,
    helpers::{get_channel_mut, player_id},
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

        // only allow if they have the channel edit permission
        let channel = get_channel_mut(eng, self.channel_id)?;
        if actor.is_player() {
            let id = player_id(actor).expect("already validated as a player");
            if let Some(member_data) = channel.get_member(id) {
                if !member_data
                    .perms
                    .contains(ChannelPermission::LoggabilityControl)
                {
                    return Err(ActionError::InsufficientPermissions);
                }
            } else {
                return Err(ActionError::InsufficientPermissions);
            }
        }

        if mutate {
            channel.loggable = self.loggable
        }

        // Broadcast the new loggability so every viewer's channel UI reflects it. Global,
        // like the initial value emitted from CreateChannel.
        ctx.push_cmd(
            Command::SetChannelLoggable {
                channel_id: self.channel_id,
                loggable: self.loggable,
            },
            CommandRecipient::System,
            eng.time,
        );

        Ok(ActionResponse::SetLoggable(SetLoggableResponse {}))
    }
}
