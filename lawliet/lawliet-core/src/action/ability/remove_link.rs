/*
* SYSTEM ACTION
* Remove a link from an ability
*/

// TODO:
// move reference counting here

use crate::{
    action::{Action, ActionInterface, ActionResponse, TryDeleteChargePool},
    helpers::{get_ability_mut, get_charge_pool, get_charge_pool_mut},
};

use crate::action::ActionActor;
pub use crate::action::{RemoveLink, RemoveLinkResponse};

impl ActionInterface for RemoveLink {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        get_charge_pool(eng, self.pool_id)?;

        let ability = get_ability_mut(eng, self.ability_id)?;
        if mutate {
            ability.remove_link(self.pool_id);
            let pool = get_charge_pool_mut(eng, self.pool_id)?;
            if pool.on_unlink() {
                Action::TryDeleteChargePool(TryDeleteChargePool { id: self.pool_id })
                    .handle(eng, ctx, actor, version, mutate)?;
            }
        }

        Ok(ActionResponse::RemoveLink(RemoveLinkResponse {}))
    }
}
