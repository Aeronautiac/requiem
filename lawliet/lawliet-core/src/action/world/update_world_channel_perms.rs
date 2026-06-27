/*
* SYSTEM ACTION
* Re-evaluate a player's permissions in every world channel based on their current modifiers
* and any per-channel overrides. Called when modifiers change.
* Skips channels the player is not a member of (e.g. explicitly removed by a host).
*/

use crate::{
    action::{Action, ActionInterface, ActionResponse, SetMember},
    actor::{ActorDisplay, player::OverrideResolver},
    channel::{ChannelMember, ChannelPermission, ChannelPermissions},
    common::ChannelKey,
    helpers::{get_actor, get_player},
};

use indexmap::IndexSet;

use crate::action::ActionActor;
pub use crate::action::{UpdateWorldChannelPerms, UpdateWorldChannelPermsResponse};

impl ActionInterface for UpdateWorldChannelPerms {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        get_player(eng, self.player_id)?;

        let player_modifiers = get_actor(eng, self.player_id)?.modifiers();
        let updates: Vec<(ChannelKey, ChannelPermissions, IndexSet<ActorDisplay>)> = {
            let player = get_player(eng, self.player_id)?;
            eng.world
                .world_channel_map
                .iter()
                .filter_map(|(name, &channel_id)| {
                    let config = eng.config.world_config.world_channels.get(name)?;
                    let over = player.get_world_channel_override(*name, OverrideResolver::Positive);

                    let base = over
                        .as_ref()
                        .map_or(config.default_perms, |o| o.default_perms);
                    let force = over
                        .as_ref()
                        .map_or(ChannelPermissions::EMPTY, |o| o.force_perms);

                    let mut blocked = ChannelPermissions::EMPTY;
                    if !(player_modifiers & config.send_blocking).is_empty() {
                        blocked |= ChannelPermission::Send;
                    }
                    if !(player_modifiers & config.view_blocking).is_empty() {
                        blocked |= ChannelPermission::View;
                    }

                    let effective = (base & !blocked) | force;
                    let displays = eng
                        .world
                        .get_channel(channel_id)?
                        .get_member(self.player_id)?
                        .displays
                        .clone();

                    Some((channel_id, effective, displays))
                })
                .collect()
        };

        for (channel_id, effective_perms, displays) in updates {
            Action::SetMember(SetMember {
                player_id: self.player_id,
                channel_id,
                settings: Some(ChannelMember {
                    perms: effective_perms,
                    displays,
                }),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::UpdateWorldChannelPerms(
            UpdateWorldChannelPermsResponse {},
        ))
    }
}
