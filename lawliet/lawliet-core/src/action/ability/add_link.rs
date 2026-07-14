/*
* SYSTEM ACTION
* Add a charge pool to the world
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    },
    helpers::{get_ability_mut, get_charge_pool, get_charge_pool_mut},
};

pub use crate::action::{AddLink, AddLinkResponse};

impl ActionInterface for AddLink {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        get_charge_pool(eng, self.pool_id)?;

        let ability = get_ability_mut(eng, self.ability_id)?;
        if mutate {
            ability.add_link(
                self.pool_id,
                self.link_type,
                self.weight,
                self.volatile,
                self.condition,
            );
            let pool = get_charge_pool_mut(eng, self.pool_id)?;
            pool.on_link();
        }

        Ok(ActionResponse::AddLink(AddLinkResponse {}))
    }
}
