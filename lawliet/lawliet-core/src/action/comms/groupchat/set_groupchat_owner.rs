/*
* SYSTEM & PLAYER ACTION
* Set the owner of a group chat
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionError, ActionInterface, ActionResponse},
    actor::modifier::Modifier,
    command::Command,
    helpers::{actor_id, get_actor, get_gc_mut, get_player},
};

use crate::action::ActionActor;
pub use crate::action::{SetGroupchatOwner, SetGroupchatOwnerResponse};

impl ActionInterface for SetGroupchatOwner {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_or_authoritative()?;

        if let Some(owner_id) = self.owner {
            get_player(eng, owner_id)?;
            let data = get_actor(eng, owner_id)?;
            if actor.is_player() && data.has_modifier(Modifier::NoContact) {
                return Err(ActionError::CannotContact);
            }
        }

        let gc = get_gc_mut(eng, self.groupchat_id)?;
        if actor.is_player() {
            let id = actor_id(actor).expect("expected valid actor id");
            if gc.owner != Some(id) {
                return Err(ActionError::NotTheOwner);
            }
        }

        let old_owner = gc.owner;
        if mutate {
            gc.set_owner(self.owner);
        }

        if let Some(old) = old_owner {
            ctx.push_cmd(
                Command::GcOwnerStatus {
                    owner: false,
                    gc_id: self.groupchat_id,
                },
                CommandRecipient::Player(old),
                eng.time,
            );
        }
        if let Some(new) = self.owner {
            ctx.push_cmd(
                Command::GcOwnerStatus {
                    owner: true,
                    gc_id: self.groupchat_id,
                },
                CommandRecipient::Player(new),
                eng.time,
            );
        }

        Ok(ActionResponse::SetGroupchatOwner(
            SetGroupchatOwnerResponse {},
        ))
    }
}
