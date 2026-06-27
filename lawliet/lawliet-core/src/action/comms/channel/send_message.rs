/*
* PLAYER ACTION
* Send a message to a channel
*/

use lawliet_types::actor::Modifier;

use crate::{
    action::{ActionError, ActionInterface, ActionResponse},
    channel::ChannelPermission,
    command::Command,
    common::BugKey,
    helpers::{get_actor, get_channel, player_id},
};

use crate::action::ActionActor;
pub use crate::action::{SendMessage, SendMessageResponse};

impl ActionInterface for SendMessage {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        _mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_only()?;
        let id = player_id(actor).expect("expected valid player id");

        let loggable = {
            let channel = get_channel(eng, self.channel_id)?;
            let member = channel.get_member(id);
            let Some(member_data) = member else {
                return Err(ActionError::NotAChannelMember);
            };
            if !member_data.perms.contains(ChannelPermission::Send) {
                return Err(ActionError::InsufficientPermissions);
            }
            if !member_data.displays.contains(&self.display) {
                return Err(ActionError::DisplayNotOwned);
            }
            channel.loggable
        };

        // this will tell the frontend to show the message to everyone who has view permissions for
        // this channel
        ctx.push_cmd(
            Command::AddMessage {
                content: self.content.clone(),
                channel_id: self.channel_id,
                sender_display: self.display,
            },
            None,
            eng.time,
        );

        // relay to all active bugs targeting this player if the channel is loggable and the player
        // does not have log LogNullification
        let actor_data = get_actor(eng, id).expect("actor should already be validated");
        if loggable && !actor_data.has_modifier(Modifier::LogNullification) {
            let bug_ids: Vec<BugKey> = eng
                .world
                .get_player(id)
                .expect("expected valid player")
                .bugs
                .iter()
                .copied()
                .collect();
            for bug_id in bug_ids {
                let bug = eng.world.get_bug(bug_id).expect("expected valid bug");
                if bug.enabled {
                    // IMPORTANT:
                    // since we're using the player's diplay here, it means that if they're posing
                    // as someone else and send a message, the message will be relayed with that
                    // fake identity (it will reveal them).
                    ctx.push_cmd(
                        Command::AddBugMessage {
                            bug_key: bug_id,
                            display: self.display,
                            content: self.content.clone(),
                        },
                        None,
                        eng.time,
                    );
                }
            }
        }

        Ok(ActionResponse::SendMessage(SendMessageResponse {}))
    }
}
