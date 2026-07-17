/*
* SYSTEM ACTION
* Create a group chat
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{Action, ActionInterface, ActionResponse, CreateChannel},
    command::Command,
    common::GroupchatKey,
    groupchat::Groupchat,
    world::ContactChannel,
};

use crate::action::ActionActor;
pub use crate::action::{CreateGroupchat, CreateGroupchatResponse};

impl ActionInterface for CreateGroupchat {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let channel_response = Action::CreateChannel(CreateChannel { loggable: true })
            .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::CreateChannel(data) = channel_response else {
            unreachable!();
        };
        let channel_id = data.id;

        let (id, contact_id) = if mutate {
            let gc_id = eng.world.add_groupchat(Groupchat::new(channel_id));
            let contact_id = eng
                .world
                .register_contact_channel(ContactChannel::Gc(gc_id));
            (gc_id, contact_id)
        } else {
            (GroupchatKey::default(), 0)
        };

        ctx.push_cmd(
            Command::MapGc {
                gc_id: id,
                channel_id,
                contact_id,
            },
            CommandRecipient::System,
            eng.time,
        );

        Ok(ActionResponse::CreateGroupchat(CreateGroupchatResponse {
            id,
        }))
    }
}
