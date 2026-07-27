/*
* SYSTEM ACTION
* Fully destroy a passive: remove from the owning actor's cache, then remove from the world.
*/

use indexmap::IndexSet;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    command::Command,
    helpers::{get_actor, get_actor_mut, get_passive, owner_view_recipient, sync_viewport},
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
        let viewport = passive.viewport;

        if let Some(owner_id) = owner {
            get_actor(eng, owner_id)?;
        }

        // Empty the log's viewport before freeing it, so everyone reading it is told they no longer
        // are rather than simply stopping.
        sync_viewport(eng, ctx, viewport, IndexSet::new(), mutate);

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
            eng.world.remove_viewport(viewport);
        }

        Ok(ActionResponse::DestroyPassive(DestroyPassiveResponse {}))
    }
}
