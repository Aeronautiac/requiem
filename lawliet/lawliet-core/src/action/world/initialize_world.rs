/*
* SYSTEM ACTION
* Initialize any necessary world state
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, AddChargePool, CreateChannel,
    },
    helpers::get_charge_pool_mut,
};

pub use crate::action::{InitializeWorld, InitializeWorldResponse};

impl ActionInterface for InitializeWorld {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let pool_config = eng.config.world_config.charge_pools.clone();
        for (name, specifier) in pool_config {
            let response = Action::AddChargePool(AddChargePool {
                base_charges: specifier.charges,
                base_reset_time: specifier.reset_time,
            })
            .handle(eng, ctx, actor, version, mutate)?;
            if mutate {
                let ActionResponse::AddChargePool(data) = response else {
                    unreachable!()
                };
                let pool = get_charge_pool_mut(eng, data.id)?;
                pool.on_link();
                eng.world.pool_map.insert(name, data.id);
            }
        }

        let channel_names: Vec<_> = eng.config.world_config.world_channels.keys().copied().collect();
        for name in channel_names {
            let response = Action::CreateChannel(CreateChannel { loggable: true })
                .handle(eng, ctx, actor, version, mutate)?;
            if mutate {
                let ActionResponse::CreateChannel(data) = response else {
                    unreachable!()
                };
                eng.world.world_channel_map.insert(name, data.id);
            }
        }

        Ok(ActionResponse::InitializeWorld(InitializeWorldResponse {}))
    }
}
