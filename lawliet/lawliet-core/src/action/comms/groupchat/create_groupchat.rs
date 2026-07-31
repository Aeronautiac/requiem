/*
* SYSTEM ACTION
* Create a group chat
*/

use crate::{
    action::{Action, ActionInterface, ActionResponse, CreateChannel},
    channel::ChannelKind,
    command::Command,
    common::GroupchatKey,
    groupchat::Groupchat,
    helpers::cmd_channel,
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

        let channel_response = Action::CreateChannel(CreateChannel {
            loggable: true,
            base_profile: None,
        })
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

        cmd_channel(
            eng,
            ctx,
            Command::MapChannel {
                channel_id,
                kind: ChannelKind::Groupchat {
                    gc_id: id,
                    contact_id,
                },
            },
            channel_id,
            false,
            None,
        );

        Ok(ActionResponse::CreateGroupchat(CreateGroupchatResponse {
            id,
        }))
    }
}
