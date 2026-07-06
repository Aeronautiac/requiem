/*
* SYSTEM ACTION
* Destroy a channel and remove it from the world.
* Callers are responsible for cleaning up any wrapper objects (lounges, groupchats, notebooks,
* world channels) that reference this channel before calling this action.
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    command::Command,
    helpers::get_channel,
};

pub use crate::action::{DestroyChannel, DestroyChannelResponse};

impl ActionInterface for DestroyChannel {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.require_system()?;
        get_channel(eng, self.channel_id)?;

        if mutate {
            eng.world.remove_channel(self.channel_id);
        }

        if !self.archive {
            ctx.push_cmd(
                Command::DeleteChannel {
                    channel_id: self.channel_id,
                },
                CommandRecipient::System,
                eng.time,
            );
        } else {
            ctx.push_cmd(
                Command::ArchiveChannel {
                    channel_id: self.channel_id,
                },
                CommandRecipient::System,
                eng.time,
            );
        }

        Ok(ActionResponse::DestroyChannel(DestroyChannelResponse {}))
    }
}
