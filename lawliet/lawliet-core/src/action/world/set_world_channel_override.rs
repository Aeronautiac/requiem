/*
* SYSTEM ACTION
* Set or clear a player's per-channel override for a specific source, then re-evaluate
* their effective permissions. Each source may hold at most one override per channel.
*/

use indexmap::IndexMap;

use crate::{
    action::{
        ActionInterface, Action, ActionResponse, UpdateWorldChannelPerms,
    },
    actor::player::{OverrideSource, SourcedWorldChannelOverride, WorldChannelOverride},
    common::ActorKey,
    config::world::WorldChannelName,
    helpers::get_player_mut,
};

use crate::action::ActionActor;
pub use crate::action::{SetWorldChannelOverride, SetWorldChannelOverrideResponse};

impl ActionInterface for SetWorldChannelOverride {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let player = get_player_mut(eng, self.player_id)?;
        if mutate {
            match &self.override_data {
                Some(data) => {
                    player
                        .world_channel_overrides
                        .entry(self.channel_name)
                        .or_insert_with(IndexMap::new)
                        .insert(self.source.clone(), SourcedWorldChannelOverride {
                            priority: self.priority,
                            data: data.clone(),
                        });
                }
                None => {
                    if let Some(channel_overrides) = player
                        .world_channel_overrides
                        .get_mut(&self.channel_name)
                    {
                        channel_overrides.swap_remove(&self.source);
                    }
                }
            }
        }

        Action::UpdateWorldChannelPerms(UpdateWorldChannelPerms {
            player_id: self.player_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::SetWorldChannelOverride(
            SetWorldChannelOverrideResponse {},
        ))
    }
}
