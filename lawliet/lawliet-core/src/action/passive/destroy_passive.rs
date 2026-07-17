/*
* SYSTEM ACTION
* Fully destroy a passive: remove from the owning actor's cache, then remove from the world.
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    command::Command,
    helpers::{get_actor, get_actor_mut, get_passive},
};

pub use crate::action::{DestroyPassive, DestroyPassiveResponse};

impl ActionInterface for DestroyPassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let passive = get_passive(eng, self.passive_id)?;
        let owner = passive.ownership_struct.owner;

        if let Some(owner_id) = owner {
            get_actor(eng, owner_id)?;
        }

        if mutate {
            if let Some(owner_id) = owner {
                get_actor_mut(eng, owner_id)
                    .expect("passive owner does not exist: engine invariant violated")
                    .remove_passive(self.passive_id);
                // drop it from the owner's observable list
                ctx.push_cmd(
                    Command::RemovePassive {
                        passive_id: self.passive_id,
                    },
                    CommandRecipient::Actor(owner_id),
                    eng.time,
                );
            }
            eng.world.remove_passive(self.passive_id);
        }

        Ok(ActionResponse::DestroyPassive(DestroyPassiveResponse {}))
    }
}
