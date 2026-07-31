// delete a profile from a channel

use lawliet_types::action::ActionResponse;
use smallvec::SmallVec;

use crate::{
    action::{ActionActor, ActionContext, ActionError, ActionInterface, ActionResult},
    common::{ActorKey, Version},
    engine::Engine,
    helpers::{
        cmd_channel_roster, cmd_profile_access, get_channel, get_channel_mut, sync_viewport,
    },
};

pub use crate::action::{DeleteProfile, DeleteProfileResponse};

impl ActionInterface for DeleteProfile {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let channel = get_channel(eng, self.channel_id)?;
        let profile = channel
            .get_profile(self.profile_id)
            .ok_or(ActionError::ProfileNotFound)?;
        let visible = profile.visible;
        // Read before the removal: afterwards there is nothing left to say who has to be told.
        let owners: SmallVec<[ActorKey; 4]> = profile.ownership.owners();

        if mutate {
            get_channel_mut(eng, self.channel_id)
                .expect("channel vanished mid-action: engine invariant violated")
                .remove_profile(self.profile_id);
        }

        // The audience is a projection of the member list, which has just moved: a former owner who
        // held View through this name alone is no longer in the room, and one who held their last
        // profile through it is no longer a member.
        let (viewport, viewers) = {
            let channel = get_channel(eng, self.channel_id)?;
            (channel.viewport, channel.viewers())
        };
        sync_viewport(eng, ctx, viewport, viewers, mutate);

        // After the resync, so it reaches exactly who can still see the channel. Nothing said
        // through the name is retracted — a name ceasing to exist is not a name that never spoke.
        if visible {
            cmd_channel_roster(eng, ctx, self.channel_id);
        }

        // Directed, so it still reaches former owners the resync has just taken out of the
        // viewport.
        for owner in owners {
            cmd_profile_access(eng, ctx, self.channel_id, owner);
        }

        Ok(ActionResponse::DeleteProfile(DeleteProfileResponse {}))
    }
}
