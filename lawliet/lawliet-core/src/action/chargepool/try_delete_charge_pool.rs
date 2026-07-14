/*
* SYSTEM ACTION
* Try to delete a charge pool (check the reference count)
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    },
    helpers::get_charge_pool,
};

pub use crate::action::{TryDeleteChargePool, TryDeleteChargePoolResponse};

impl ActionInterface for TryDeleteChargePool {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        let pool = get_charge_pool(eng, self.id)?;

        if mutate && pool.ref_count == 0 {
            eng.world.remove_charge_pool(self.id);
        }

        Ok(ActionResponse::TryDeleteChargePool(
            TryDeleteChargePoolResponse {},
        ))
    }
}
