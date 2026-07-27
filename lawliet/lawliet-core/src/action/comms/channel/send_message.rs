/*
* Authoritative and Player Action
* Send a message to a channel
*
* A message on a loggable channel is addressed to the channel's membership viewport, to the two log
* viewports that make up the record (the sender's and the channel's), and to every enabled bug
* watching the sender. Modifier::LogNullification suppresses everything but the membership
* viewport: the room genuinely heard it, and that cannot be taken back — being off the record only
* ever means staying out of the record.
*
* A player message also ends a trial's grace subphase when the sender is the side holding the
* floor — the one place a message drives something other than itself.
*/

use lawliet_types::{actor::Modifier, command::CommandRecipient};

use crate::{
    action::{
        Action, ActionError, ActionInterface, ActionResponse, AdvanceProsecution,
        prosecution::grace_ended_by,
    },
    channel::ChannelPermission,
    command::Command,
    common::BugKey,
    helpers::{cmd_channel, get_actor, get_channel, player_id},
};

use crate::action::ActionActor;
pub use crate::action::{SendMessage, SendMessageResponse};

impl ActionInterface for SendMessage {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_or_authoritative()?;

        // Set only for a player message that starts a trial slot; see below.
        let mut grace_ended = None;

        if actor.is_player() {
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

            let actor_data = get_actor(eng, id).expect("actor should already be validated");
            if loggable && !actor_data.has_modifier(Modifier::LogNullification) {
                // The record, written twice because two abilities ask two different questions of
                // it. An autopsy asks what a given player said, so it reads the sender's log
                // viewport; a tap-in asks what was said in a given channel, so it reads the
                // channel's. Neither can be answered from the other: sender_display may be a lie,
                // and a channel's live viewport keeps messages this branch is suppressing.
                let sender_log = eng
                    .world
                    .get_player(id)
                    .expect("expected valid player")
                    .log_viewport;
                let channel_log = get_channel(eng, self.channel_id)
                    .expect("channel already validated")
                    .log_viewport;
                for viewport in [sender_log, channel_log] {
                    ctx.push_cmd(
                        Command::AddMessage {
                            content: self.content.clone(),
                            channel_id: self.channel_id,
                            sender_display: self.display,
                        },
                        CommandRecipient::Viewport(viewport),
                        eng.time,
                    );
                }

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
                        // A borrowed display is relayed as-is, so posing as someone else and then
                        // speaking on a bugged channel exposes the pose to whoever is listening.
                        let bug_viewport = bug.viewport;
                        ctx.push_cmd(
                            Command::AddBugMessage {
                                bug_key: bug_id,
                                display: self.display,
                                content: self.content.clone(),
                            },
                            CommandRecipient::Viewport(bug_viewport),
                            eng.time,
                        );
                    }
                }
            }

            grace_ended = grace_ended_by(eng, self.channel_id, id);
        }

        // Addressed to the channel, which is precisely everyone holding View on it — and, on
        // entry, anyone granted View later.
        cmd_channel(
            eng,
            ctx,
            Command::AddMessage {
                content: self.content.clone(),
                channel_id: self.channel_id,
                sender_display: self.display,
            },
            self.channel_id,
        );

        // After the message, so the thing that started the slot is on the wire before the slot
        // reports itself as started.
        if let Some(prosecution_id) = grace_ended {
            Action::AdvanceProsecution(AdvanceProsecution { prosecution_id }).handle(
                eng,
                ctx,
                &ActionActor::System,
                version,
                mutate,
            )?;
        }

        Ok(ActionResponse::SendMessage(SendMessageResponse {}))
    }
}
