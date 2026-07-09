/*
* SYSTEM ACTION
* Map a player ID to a channel member struct within the channel
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionInterface, ActionResponse},
    actor::ActorDisplay,
    command::Command,
    helpers::{get_channel_mut, get_player},
};

use crate::action::ActionActor;
pub use crate::action::{SetMember, SetMemberResponse};

// Only Raw and Role displays are surfaced as channel members. Mysterious/System (and Org,
// which the frontend doesn't model yet) name no real participant, so they're never sent as
// member updates.
fn renders_as_member(display: &ActorDisplay) -> bool {
    matches!(display, ActorDisplay::Raw(_) | ActorDisplay::Role(_))
}

impl ActionInterface for SetMember {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        get_player(eng, self.player_id)?;

        let time = eng.time;
        let channel = get_channel_mut(eng, self.channel_id)?;

        // For a removal, capture the leaver's displays before set_member clears them, so
        // the remaining members can be told exactly who left.
        let removed_displays = if self.settings.is_none() {
            channel
                .members
                .get(&self.player_id)
                .map(|m| m.displays.clone())
        } else {
            None
        };

        if mutate {
            channel.set_member(self.player_id, self.settings.clone());
        }

        if let Some(member) = &self.settings {
            // Under the current ruleset a member holds exactly one set of displays per
            // channel, identical to every viewer, so the same ShowChannelMember commands
            // are broadcast to everyone. If per-viewer display divergence (deception) is
            // ever added, display resolution will have to become recipient-aware here.
            //
            // Perms must reach the frontend before the members: it treats a channel entry
            // (perms) as the membership signal, and ShowChannelMember only writes into an
            // existing entry. Commands are delivered in push order, so emit perms first.
            ctx.push_cmd(
                Command::UpdateChannelView {
                    channel_id: self.channel_id,
                    displays: member.displays.clone(),
                    perms: member.perms,
                },
                CommandRecipient::Player(self.player_id),
                time,
            );

            // note that the player is sent a member display command for their own displays as well
            // this is just more convenient than having to derive it on the frontend + it would require
            // extra backend logic
            for (_, member) in channel.members.iter() {
                for display in member.displays.iter().filter(|&d| renders_as_member(d)) {
                    ctx.push_cmd(
                        Command::ShowChannelMember {
                            channel_id: self.channel_id,
                            display: *display,
                            channel_perms: member.perms,
                        },
                        CommandRecipient::Player(self.player_id),
                        time,
                    );
                }
            }

            // Tell the players already in the channel about the newcomer. Skip the
            // newcomer itself — it was covered by the roster loop above. Existing members
            // already hold a channel entry, so their ShowChannelMember lands.
            for (id, _) in channel.members.iter() {
                if *id == self.player_id {
                    continue;
                }
                for display in member.displays.iter().filter(|&d| renders_as_member(d)) {
                    ctx.push_cmd(
                        Command::ShowChannelMember {
                            channel_id: self.channel_id,
                            display: *display,
                            channel_perms: member.perms,
                        },
                        CommandRecipient::Player(*id),
                        time,
                    );
                }
            }
        } else {
            for (_, member) in channel.members.iter() {
                for display in member.displays.iter().filter(|&d| renders_as_member(d)) {
                    ctx.push_cmd(
                        Command::RemoveChannelMember {
                            channel_id: self.channel_id,
                            display: *display,
                        },
                        CommandRecipient::Player(self.player_id),
                        time,
                    );
                }
            }

            ctx.push_cmd(
                Command::RemoveChannel {
                    channel_id: self.channel_id,
                },
                CommandRecipient::Player(self.player_id),
                time,
            );

            // Tell the remaining members that the leaver is gone. By now the leaver has
            // been removed from channel.members (execute pass), so this only reaches the
            // others.
            if let Some(displays) = &removed_displays {
                for (id, _) in channel.members.iter() {
                    if *id == self.player_id {
                        continue;
                    }
                    for display in displays.iter().filter(|&d| renders_as_member(d)) {
                        ctx.push_cmd(
                            Command::RemoveChannelMember {
                                channel_id: self.channel_id,
                                display: *display,
                            },
                            CommandRecipient::Player(*id),
                            time,
                        );
                    }
                }
            }
        }

        Ok(ActionResponse::SetMember(SetMemberResponse {}))
    }
}
