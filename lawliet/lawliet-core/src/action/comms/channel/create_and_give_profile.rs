/*
* SYSTEM ACTION
* Create a profile and hand it to a player in one step, which is also what makes them a member.
*/

use lawliet_types::action::ActionResponse;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResult, AddProfile,
        SetProfileAccess,
    },
    common::Version,
    engine::Engine,
    helpers::get_player,
};

pub use crate::action::{CreateAndGiveProfile, CreateAndGiveProfileResponse};

impl ActionInterface for CreateAndGiveProfile {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let response = Action::AddProfile(AddProfile {
            channel_id: self.channel_id,
            display: self.display,
            visible: self.visible,
            shared: self.shared,
            transferrable: self.transferrable,
            perm_policy: self.perm_policy,
        })
        .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::AddProfile(data) = response else {
            unreachable!()
        };

        // The player is checked on both passes; the grant itself only happens on the second. A key
        // AddProfile has not issued yet names nothing, and asking the grant about it would fail for
        // a reason that is not real. There is nothing lost: a name that has just been made is worn
        // by nobody, so the one thing the grant can refuse cannot arise here.
        get_player(eng, self.player_id)?;
        if mutate {
            Action::SetProfileAccess(SetProfileAccess {
                channel_id: self.channel_id,
                profile_id: data.profile_id,
                player_id: self.player_id,
                granted: true,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::CreateAndGiveProfile(
            CreateAndGiveProfileResponse {
                profile_id: data.profile_id,
            },
        ))
    }
}
