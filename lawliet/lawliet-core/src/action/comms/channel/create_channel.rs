/*
* SYSTEM ACTION
* Create a channel
*/

use crate::{
    action::{
        ActionInterface, ActionResponse,
    },
    channel::Channel,
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
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let id = if mutate {
            eng.world.add_channel(Channel::new(self.loggable))
        } else {
            ChannelKey::default()
        };

        Ok(ActionResponse::CreateChannel(CreateChannelResponse { id }))
    }
}
