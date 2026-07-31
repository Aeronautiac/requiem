/*
* SYSTEM ACTION
* Take a player out of a channel entirely.
*
* Each name they hold is disposed of by whether it could ever have belonged to anybody else. A name
* that could not is destroyed, because it had no life without them; a name that could is taken off
* them and left in the channel for whoever comes next.
*/

use lawliet_types::action::ActionResponse;
use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResult, DeleteProfile,
        SetProfileAccess,
    },
    common::{ProfileKey, Version},
    engine::Engine,
    helpers::get_channel,
};

pub use crate::action::{RemoveFromChannel, RemoveFromChannelResponse};

impl ActionInterface for RemoveFromChannel {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Collected first, with each name's fate decided here: every step below edits the very set
        // being walked.
        let channel = get_channel(eng, self.channel_id)?;
        let profiles: SmallVec<[(ProfileKey, bool); 4]> = channel
            .get_member(self.player_id)
            .map(|member| {
                member
                    .profiles
                    .iter()
                    .filter_map(|id| Some((*id, channel.get_profile(*id)?.transferrable)))
                    .collect()
            })
            .unwrap_or_default();

        for (profile_id, transferrable) in profiles {
            if transferrable {
                Action::SetProfileAccess(SetProfileAccess {
                    channel_id: self.channel_id,
                    profile_id,
                    player_id: self.player_id,
                    granted: false,
                })
                .handle(eng, ctx, actor, version, mutate)?;
            } else {
                Action::DeleteProfile(DeleteProfile {
                    channel_id: self.channel_id,
                    profile_id,
                })
                .handle(eng, ctx, actor, version, mutate)?;
            }
        }

        Ok(ActionResponse::RemoveFromChannel(
            RemoveFromChannelResponse {},
        ))
    }
}
