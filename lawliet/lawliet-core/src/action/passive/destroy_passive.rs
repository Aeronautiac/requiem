/*
* SYSTEM ACTION
* Fully destroy a passive: remove from the owning actor's cache, then remove from the world.
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    },
    common::PassiveKey,
    helpers::{get_passive, get_actor, get_actor_mut},
};

pub use crate::action::{DestroyPassive, DestroyPassiveResponse};

impl ActionInterface for DestroyPassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
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
            }
            eng.world.remove_passive(self.passive_id);
        }

        Ok(ActionResponse::DestroyPassive(DestroyPassiveResponse {}))
    }
}
