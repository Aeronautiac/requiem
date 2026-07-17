/*
* SYSTEM ACTION
* Add a player to every world channel with no permissions, then evaluate their starting
* permissions via UpdateWorldChannelPerms.
*/

use indexmap::indexset;

use crate::{
    action::{Action, ActionInterface, ActionResponse, SetMember, UpdateWorldChannelPerms},
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermissions},
    common::ChannelKey,
    helpers::get_player,
};

use crate::action::ActionActor;
pub use crate::action::{AddToWorldChannels, AddToWorldChannelsResponse};

impl ActionInterface for AddToWorldChannels {
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

        let channel_ids: Vec<ChannelKey> = eng.world.world_channel_map.values().copied().collect();

        for channel_id in channel_ids {
            Action::SetMember(SetMember {
                player_id: self.player_id,
                channel_id,
                settings: Some(ChannelMember {
                    perms: ChannelPermissions::EMPTY,
                    displays: indexset![ActorDisplay::Raw(self.player_id)],
                }),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Action::UpdateWorldChannelPerms(UpdateWorldChannelPerms {
            player_id: self.player_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::AddToWorldChannels(
            AddToWorldChannelsResponse {},
        ))
    }
}
