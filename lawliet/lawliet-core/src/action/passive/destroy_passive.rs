/*
* SYSTEM ACTION
* Fully destroy a passive: remove from the owning actor's cache, then remove from the world.
*/

use lawliet_types::command::Command;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        UpdateContactLogViewports,
    },
    helpers::{get_actor, get_actor_mut, get_passive, owner_view_recipient},
};

pub use crate::action::{DestroyPassive, DestroyPassiveResponse};

impl ActionInterface for DestroyPassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
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
                    owner_view_recipient(eng, owner_id),
                    eng.time,
                );
            }
            eng.world.remove_passive(self.passive_id);

            // A contact-log reader reached the record only through effective possession of this
            // passive; with it gone, recompute the world log viewports so anyone who has lost their
            // last route into one is exited from it. The record itself outlives the passive.
            Action::UpdateContactLogViewports(UpdateContactLogViewports {})
                .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::DestroyPassive(DestroyPassiveResponse {}))
    }
}
