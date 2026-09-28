/*
* SYSTEM ACTION
* Create a channel
*/

use crate::{
    action::{ActionInterface, ActionResponse},
    channel::Channel,
    common::ChannelKey,
    helpers::open_viewport,
    viewport::ViewportKind,
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

        // The channel owns its viewport for its whole life; DestroyChannel frees it. That lives in
        // an action rather than World::add_channel/remove_channel so the allocation sits next to
        // the `mutate` gate that governs it instead of inheriting it invisibly.
        //
        // The log is claimed the same way but never given back. A viewport is an audience and dies
        // with the thing it was an audience for; a record is what was said, and outlives it.
        //
        // Nothing is announced here. The caller announces the channel through `map_channel` once
        // it knows what the channel belongs to.
        let id = if mutate {
            let viewport = open_viewport(eng, ctx, ViewportKind::Channel);
            let log = eng.world.add_log();
            eng.world.add_channel(Channel::new(
                self.loggable,
                viewport,
                log,
                self.base_profile,
            ))
        } else {
            ChannelKey::default()
        };

        Ok(ActionResponse::CreateChannel(CreateChannelResponse { id }))
    }
}
