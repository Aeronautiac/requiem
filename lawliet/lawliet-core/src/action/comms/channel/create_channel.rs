/*
* SYSTEM ACTION
* Create a channel
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionInterface, ActionResponse},
    channel::Channel,
    command::Command,
    common::ChannelKey,
};

use crate::action::ActionActor;
pub use crate::action::{CreateChannel, CreateChannelResponse};

impl ActionInterface for CreateChannel {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let id = if mutate {
            eng.world.add_channel(Channel::new(self.loggable))
        } else {
            ChannelKey::default()
        };

        // Announce the channel's initial loggability. The frontend stores this keyed by
        // channel id independently of the Map* command that establishes the channel, so
        // emission order relative to that Map doesn't matter.
        ctx.push_cmd(
            Command::SetChannelLoggable {
                channel_id: id,
                loggable: self.loggable,
            },
            CommandRecipient::System,
            eng.time,
        );

        Ok(ActionResponse::CreateChannel(CreateChannelResponse { id }))
    }
}
