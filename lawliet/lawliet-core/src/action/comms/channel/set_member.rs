/*
* SYSTEM ACTION
* Map a player ID to a channel member struct within the channel
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionInterface, ActionResponse},
    actor::ActorDisplay,
    command::Command,
    helpers::{get_channel_mut, get_player, sync_viewport},
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

        // The member list is the authority; the viewport is a projection of it. Resync before
        // emitting anything, so a newcomer already has access when the roster below lands and a
        // leaver has already stopped receiving.
        let viewport = channel.viewport;
        let viewers = channel.viewers();
        sync_viewport(eng, ctx, viewport, viewers, mutate);

        if let Some(member) = &self.settings {
            // Perms stay directed: View/Send/LoggabilityControl are per-viewer by nature, and
            // the frontend treats a channel entry (perms) as the membership signal. Commands
            // are delivered in push order, so this must precede the roster.
            ctx.push_cmd(
                Command::UpdateChannelView {
                    channel_id: self.channel_id,
                    displays: member.displays.clone(),
                    perms: member.perms,
                },
                CommandRecipient::Actor(self.player_id),
                time,
            );

            // The roster is addressed to the viewport rather than sent to each member in turn.
            // Under the current ruleset a member holds one set of displays per channel,
            // identical to every viewer, so there was never anything per-viewer about these:
            // the old code sent the whole existing roster to the newcomer and then the
            // newcomer to everyone else, which is the same content assembled twice. Backfill
            // covers the newcomer's copy of the existing roster, so only their own entry is
            // left to emit.
            //
            // If per-viewer display divergence (deception) is ever added, this has to go back
            // to being resolved per recipient.
            for display in member.displays.iter().filter(|&d| renders_as_member(d)) {
                ctx.push_cmd(
                    Command::ShowChannelMember {
                        channel_id: self.channel_id,
                        display: *display,
                        channel_perms: member.perms,
                    },
                    CommandRecipient::Viewport(viewport),
                    time,
                );
            }
        } else if let Some(displays) = &removed_displays {
            // The leaver exited the viewport in the resync above, so this reaches the
            // remaining members only — exactly who needs it.
            for display in displays.iter().filter(|&d| renders_as_member(d)) {
                ctx.push_cmd(
                    Command::RemoveChannelMember {
                        channel_id: self.channel_id,
                        display: *display,
                    },
                    CommandRecipient::Viewport(viewport),
                    time,
                );
            }
        }

        Ok(ActionResponse::SetMember(SetMemberResponse {}))
    }
}
