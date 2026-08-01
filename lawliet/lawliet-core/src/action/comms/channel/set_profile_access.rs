use lawliet_types::action::ActionResponse;

use crate::{
    action::{ActionActor, ActionContext, ActionError, ActionInterface, ActionResult},
    channel::ProfileOwnership,
    common::Version,
    engine::Engine,
    helpers::{
        cmd_channel_roster, cmd_profile_access, get_channel, get_channel_mut, get_player,
        sync_viewport,
    },
};

pub use crate::action::{SetProfileAccess, SetProfileAccessResponse};

impl ActionInterface for SetProfileAccess {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        get_player(eng, self.player_id)?;

        let profile = get_channel(eng, self.channel_id)?
            .get_profile(self.profile_id)
            .ok_or(ActionError::ProfileNotFound)?;

        // Refused rather than quietly ignored, and on both passes. A name only one person can be
        // wearing is the whole reason wearing it means anything, so handing it to a second one is
        // a mistake worth hearing about rather than a no-op to route around.
        let worn_by_another = matches!(
            &profile.ownership,
            ProfileOwnership::Single(Some(held)) if *held != self.player_id
        );
        if self.granted && worn_by_another {
            return Err(ActionError::ProfileNotShareable);
        }

        if mutate {
            let channel = get_channel_mut(eng, self.channel_id)
                .expect("channel vanished mid-action: engine invariant violated");
            if self.granted {
                channel.grant_profile(self.player_id, self.profile_id);
            } else {
                channel.revoke_profile(self.player_id, self.profile_id);
            }
        }

        // Membership and audience both follow from who holds what, and both may have just moved:
        // a first name makes someone a member, and a last one taken away ends it.
        let (viewport, viewers) = {
            let channel = get_channel(eng, self.channel_id)?;
            (channel.viewport, channel.viewers())
        };
        sync_viewport(eng, ctx, viewport, viewers, mutate);

        // Somebody may have just gained sight of the channel, and a roster is never in the
        // viewport's history for them to be handed on the way in.
        cmd_channel_roster(eng, ctx, self.channel_id);
        cmd_profile_access(eng, ctx, self.channel_id, self.player_id);

        Ok(ActionResponse::SetProfileAccess(
            SetProfileAccessResponse {},
        ))
    }
}
