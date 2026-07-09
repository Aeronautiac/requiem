/*
* SYSTEM & PLAYER ACTION
* Remove a player from a group chat
*/

use crate::{
    action::{Action, ActionError, ActionInterface, ActionResponse, SetMember},
    helpers::{actor_id, get_gc, get_gc_mut, get_player_mut},
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

        let gc = get_gc(eng, self.groupchat_id)?;
        if actor.is_player() {
            let id = actor_id(actor).expect("expected valid actor id");
            if gc.owner != Some(id) {
                return Err(ActionError::NotTheOwner);
            }
        }

        if !gc.contains_member(self.player_id) {
            return Err(ActionError::NotInGroupchat);
        }
        let channel_id = gc.channel_id;

        // Drop the channel membership too (emits RemoveChannel to the player and
        // RemoveChannelMember to the others), mirroring remove_from_lounge. Without
        // this the player would keep seeing the gc channel after being removed.
        Action::SetMember(SetMember {
            player_id: self.player_id,
            channel_id,
            settings: None,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        if mutate {
            let gc = get_gc_mut(eng, self.groupchat_id)?;
            gc.remove_member(self.player_id);

            let player_data = get_player_mut(eng, self.player_id)?;
            player_data.remove_groupchat(self.groupchat_id);
        }

        Ok(ActionResponse::RemoveFromGroupchat(
            RemoveFromGroupchatResponse {},
        ))
    }
}
