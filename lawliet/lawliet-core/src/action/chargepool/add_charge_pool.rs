/*
* SYSTEM ACTION
* Add a charge pool to the world
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    },
    chargepool::ChargePool,
    common::{ChargeCount, ChargePoolKey, IterationCount},
};

pub use crate::action::{AddChargePool, AddChargePoolResponse};

impl ActionInterface for AddChargePool {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let id = if mutate {
            let pool = ChargePool::new(self.base_charges, self.base_reset_time);
            eng.world.add_charge_pool(pool)
        } else {
            ChargePoolKey::default()
        };

        Ok(ActionResponse::AddChargePool(AddChargePoolResponse { id }))
    }
}
