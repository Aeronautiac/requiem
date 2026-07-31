/*
* SYSTEM ACTION
* Change the rule deciding what a name permits.
*
* Refused if the policy cannot answer for this profile, and refused is all it is: the profile is
* left exactly as it was, still holding the rule it had. Construction is the only place an
* incompatible policy destroys anything, because there the alternative is a profile that was never
* coherent to begin with.
*
* The permissions themselves are not written here. They follow from the next UpdateChannels, like
* everything else a policy decides.
*/

use lawliet_types::action::ActionResponse;

use crate::{
    action::{ActionActor, ActionContext, ActionError, ActionInterface, ActionResult},
    channel::policies::IPermUpdatePolicy,
    common::Version,
    engine::Engine,
    helpers::{get_channel, get_channel_mut},
};

pub use crate::action::{SetProfilePolicy, SetProfilePolicyResponse};

impl ActionInterface for SetProfilePolicy {
    fn handle(
        &mut self,
        eng: &mut Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let channel = get_channel(eng, self.channel_id)?;
        let profile = channel
            .get_profile(self.profile_id)
            .ok_or(ActionError::ProfileNotFound)?;
        if !self.perm_policy.fits(eng, channel, profile) {
            return Err(ActionError::IncompatiblePolicy);
        }

        if mutate {
            get_channel_mut(eng, self.channel_id)
                .expect("channel vanished mid-action: engine invariant violated")
                .get_profile_mut(self.profile_id)
                .expect("profile validated above no longer exists: engine invariant violated")
                .perm_policy = self.perm_policy;
        }

        Ok(ActionResponse::SetProfilePolicy(SetProfilePolicyResponse {}))
    }
}
