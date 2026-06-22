/*
* SYSTEM & PLAYER ACTION
* Remove a player from a group chat
*/

use crate::{
    action::{
        ActionInterface, ActionError, ActionResponse,
    },
    common::{ActorKey, GroupchatKey},
    helpers::{actor_id, get_gc_mut, get_player_mut},
};

use crate::action::ActionActor;
pub use crate::action::{RemoveFromGroupchat, RemoveFromGroupchatResponse};

impl ActionInterface for RemoveFromGroupchat {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_or_authoritative()?;

        let gc = get_gc_mut(eng, self.groupchat_id)?;
        if actor.is_player() {
            let id = actor_id(actor).expect("expected valid actor id");
            if gc.owner != Some(id) {
                return Err(ActionError::NotTheOwner);
            }
        }

        if !gc.contains_member(self.player_id) {
            return Err(ActionError::NotInGroupchat);
        }
        if mutate {
            gc.remove_member(self.player_id);
        }

        let player_data = get_player_mut(eng, self.player_id)?;
        if mutate {
            player_data.remove_groupchat(self.groupchat_id);
        }

        Ok(ActionResponse::RemoveFromGroupchat(
            RemoveFromGroupchatResponse {},
        ))
    }
}
