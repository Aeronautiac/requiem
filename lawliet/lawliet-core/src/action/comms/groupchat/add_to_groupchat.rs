/*
* SYSTEM & PLAYER ACTION
* Add a player to a group chat
*/

use crate::{
    action::{
        ActionInterface, Action, ActionError, ActionResponse, SetGroupchatOwner,
    },
    actor::modifier::Modifier,
    common::{ActorKey, GroupchatKey},
    helpers::{actor_id, get_actor, get_actor_mut, get_gc, get_gc_mut, get_player_mut},
};

// make sure to keep the player's caches up to date as well

use crate::action::ActionActor;
pub use crate::action::{AddToGroupchat, AddToGroupchatResponse};

impl ActionInterface for AddToGroupchat {
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

            let actor_data = get_actor(eng, id).expect("expected valid actor");
            if actor_data.has_modifier(Modifier::NoContact) {
                return Err(ActionError::CannotContact);
            }
        }

        let target_data = get_actor_mut(eng, self.player_id)?;
        if target_data.has_modifier(Modifier::NoContact) {
            return Err(ActionError::CannotContact);
        }

        let gc = get_gc_mut(eng, self.groupchat_id)?;
        let channel_id = gc.channel_id;
        if mutate {
            gc.add_member(self.player_id);
        }

        let player_data = get_player_mut(eng, self.player_id)?;
        if mutate {
            player_data.add_groupchat(self.groupchat_id);
        }

        if self.owner {
            Action::SetGroupchatOwner(SetGroupchatOwner {
                groupchat_id: self.groupchat_id,
                owner: Some(self.player_id),
            })
            .handle(
                eng,
                ctx,
                &ActionActor::System,
                version,
                mutate,
            )?;
        }

        Ok(ActionResponse::AddToGroupchat(AddToGroupchatResponse {}))
    }
}
