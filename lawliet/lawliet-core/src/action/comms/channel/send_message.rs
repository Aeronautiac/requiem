/*
* Authoritative and Player Action
* Send a message to a channel
*
* A message on a loggable channel is addressed to the channel's viewport, to the two records that
* make it up (the sender's and the channel's), and to every enabled bug watching the sender.
* Modifier::LogNullification suppresses everything but the viewport: the room genuinely heard it,
* and that cannot be taken back — being off the record only ever means staying out of the record.
*
* You speak as a PROFILE, not as yourself, and Send belongs to the profile: the same person may be
* able to talk here under one of their names and not another. Speaking through a name the room has
* not been told about reveals it, immediately before the message that revealed it.
*
* A player message also ends a trial's grace subphase when the sender is the side holding the
* floor — the one place a message drives something other than itself.
*/

use lawliet_types::{actor::Modifier, channel::ChannelPerm, command::CommandRecipient};
use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionError, ActionInterface, ActionResponse, AdvanceProsecution,
        prosecution::grace_ended_by,
    },
    actor::ActorDisplay,
    command::Command,
    common::BugKey,
    helpers::{
        cmd_channel, cmd_channel_roster, get_actor, get_channel, get_channel_mut, player_id,
    },
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
        let sender = player_id(actor);

        // Who the room will see said this, and whether the sender is entitled to be them. A host
        // speaking as nobody shows as System and holds no name here; anyone else must name one
        // they hold, and Send is asked of the name rather than of the person.
        let display = match self.profile_id {
            Some(profile_id) => {
                let profile = get_channel(eng, self.channel_id)?
                    .get_profile(profile_id)
                    .ok_or(ActionError::ProfileNotFound)?;
                if !profile.perms.contains(ChannelPerm::Send) {
                    return Err(ActionError::InsufficientPermissions);
                }
                if let Some(id) = sender
                    && !profile.ownership.contains(id)
                {
                    return Err(ActionError::ProfileNotOwned);
                }
                profile.display
            }
            None => {
                if actor.is_player() {
                    return Err(ActionError::ProfileRequired);
                }
                ActorDisplay::System
            }
        };

        // Speaking through a name does two unrelated things at once. It adds the speaker to the
        // profile's list of everyone who has ever used it, which is what a public kidnapping reads
        // later to name whoever talked. And, separately, it makes the name visible if the room did
        // not know it existed, which is about the name and not about who wore it.
        let mut revealed = false;
        if mutate && let (Some(profile_id), Some(id)) = (self.profile_id, sender) {
            let channel = get_channel_mut(eng, self.channel_id)
                .expect("channel vanished mid-action: engine invariant violated");
            if let Some(profile) = channel.get_profile_mut(profile_id) {
                revealed = profile.on_send(id);
            }
        }

        if let Some(id) = sender {
            let loggable = get_channel(eng, self.channel_id)?.loggable;
            let actor_data = get_actor(eng, id).expect("actor should already be validated");

            if loggable && !actor_data.has_modifier(Modifier::LogNullification) {
                // The SENDER's half of the record, which cmd_channel cannot write because it is not
                // about the channel at all. An autopsy asks what a given player said, wherever they
                // said it, so it reads this; a tap-in asks what was said in a given channel, so it
                // reads the one cmd_channel writes. Neither answers the other: sender_display may
                // be a lie, but the record a message arrived on cannot be.
                let sender_log = eng.world.get_player(id).expect("expected valid player").log;
                ctx.push_cmd(
                    Command::AddMessage {
                        content: self.content.clone(),
                        channel_id: self.channel_id,
                        sender_display: display,
                    },
                    CommandRecipient::Log(sender_log),
                    eng.time,
                );

                let bug_ids: SmallVec<[BugKey; 8]> = eng
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
                        // A borrowed name is relayed as-is, so wearing someone else's and then
                        // speaking on a bugged channel exposes the pose to whoever is listening.
                        let bug_viewport = bug.viewport;
                        ctx.push_cmd(
                            Command::AddBugMessage {
                                bug_key: bug_id,
                                display,
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

        // Before the message, so a name the room is meeting for the first time is on the roster by
        // the time something is attributed to it.
        if revealed {
            cmd_channel_roster(eng, ctx, self.channel_id);
        }

        cmd_channel(
            eng,
            ctx,
            Command::AddMessage {
                content: self.content.clone(),
                channel_id: self.channel_id,
                sender_display: display,
            },
            self.channel_id,
            true,
            sender,
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
