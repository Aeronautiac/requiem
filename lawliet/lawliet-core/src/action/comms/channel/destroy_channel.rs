/*
* SYSTEM ACTION
* Destroy a channel and remove it from the world.
* Callers are responsible for cleaning up any wrapper objects (lounges, groupchats, notebooks,
* world channels) that reference this channel before calling this action.
*/

use lawliet_types::command::CommandRecipient;

use indexmap::IndexSet;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    command::Command,
    helpers::{get_channel, sync_viewport},
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
        let channel = get_channel(eng, self.channel_id)?;
        let viewport = channel.viewport;

        // Order matters and is easy to get wrong: the archival notice is addressed to the very
        // viewport being torn down, so it must be emitted while the members are still in it.
        // Announce, then exit everyone, then free. Getting this backwards leaves a channel
        // sitting in every client forever with no indication it ended.
        ctx.push_cmd(
            Command::ArchiveChannel {
                channel_id: self.channel_id,
            },
            CommandRecipient::Viewport(viewport),
            eng.time,
        );

        sync_viewport(eng, ctx, viewport, IndexSet::new(), mutate);

        // The record is not freed with the channel. A viewport is an audience and has nobody left
        // once the room is gone; a log is what was said in it, and a tap-in or an autopsy may still
        // ask about a channel that no longer exists.
        if mutate {
            eng.world.remove_channel(self.channel_id);
            eng.world.remove_viewport(viewport);
        }

        Ok(ActionResponse::DestroyChannel(DestroyChannelResponse {}))
    }
}
