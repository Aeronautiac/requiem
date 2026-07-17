/*
* SYSTEM ACTION
* Add charges to a pool
*/

use crate::{
    action::{ActionInterface, ActionResponse},
    helpers::get_charge_pool_mut,
};

use crate::action::ActionActor;
pub use crate::action::{AddCharges, AddChargesResponse};

impl ActionInterface for AddCharges {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let pool = get_charge_pool_mut(eng, self.id)?;
        if mutate {
            pool.add_charges(self.charges);
        }

        Ok(ActionResponse::AddCharges(AddChargesResponse {}))
    }
}
