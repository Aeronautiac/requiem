/*
* SYSTEM ACTION
* Remove a player from a lounge
*/

use crate::{
    action::{Action, ActionError, ActionInterface, ActionResponse, SetMember},
    helpers::{get_lounge, get_player, get_player_mut},
};

use crate::action::ActionActor;
pub use crate::action::{RemoveFromLounge, RemoveFromLoungeResponse};

impl ActionInterface for RemoveFromLounge {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let player = get_player(eng, self.player_id)?;
        if !player.lounges.contains(&self.lounge_id) {
            return Err(ActionError::PlayerNotInLounge);
        }

        let lounge = get_lounge(eng, self.lounge_id)?;
        Action::SetMember(SetMember {
            player_id: self.player_id,
            channel_id: lounge.channel_id,
            settings: None,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        if mutate {
            let player = get_player_mut(eng, self.player_id)?;
            player.remove_lounge(self.lounge_id);
        }

        Ok(ActionResponse::RemoveFromLounge(
            RemoveFromLoungeResponse {},
        ))
    }
}
