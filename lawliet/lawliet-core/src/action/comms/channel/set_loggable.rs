/*
* SYSTEM ACTION
* Set the loggable status of a channel
*/

use crate::{
    action::{
        ActionInterface, ActionResponse,
    },
    common::ChannelKey,
    helpers::get_channel_mut,
};

use crate::action::ActionActor;
pub use crate::action::{SetLoggable, SetLoggableResponse};

impl ActionInterface for SetLoggable {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let channel = get_channel_mut(eng, self.channel_id)?;
        if mutate {
            channel.loggable = self.loggable
        }

        // TODO:
        // host command(s)

        Ok(ActionResponse::SetLoggable(SetLoggableResponse {}))
    }
}
