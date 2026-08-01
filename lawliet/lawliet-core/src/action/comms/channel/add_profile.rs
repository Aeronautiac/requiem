// create and add a profile to a channel

use lawliet_types::{action::ActionResponse, common::ProfileKey};

use crate::{
    action::{ActionActor, ActionContext, ActionError, ActionInterface, ActionResult},
    channel::{ChannelProfile, policies::IPermUpdatePolicy},
    common::Version,
    engine::Engine,
    helpers::{cmd_channel_roster, get_channel, get_channel_mut},
};

pub use crate::action::{AddProfile, AddProfileResponse};

impl ActionInterface for AddProfile {
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
        let profile = ChannelProfile::new(
            self.display,
            self.visible,
            self.shared,
            self.transferrable,
            self.perm_policy,
        );
        if !self.perm_policy.fits(eng, channel, &profile) {
            return Err(ActionError::IncompatiblePolicy);
        }

        let id = if mutate {
            get_channel_mut(eng, self.channel_id)
                .expect("channel vanished mid-action: engine invariant violated")
                .profiles
                .insert(profile)
        } else {
            ProfileKey::default()
        };

        // A name the room can already see changes the roster. One that starts hidden does not: the
        // room is not told it exists until the message that reveals it.
        if self.visible {
            cmd_channel_roster(eng, ctx, self.channel_id);
        }

        Ok(ActionResponse::AddProfile(AddProfileResponse {
            profile_id: id,
        }))
    }
}
