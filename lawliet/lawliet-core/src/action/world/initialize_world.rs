/*
* SYSTEM ACTION
* Initialize any necessary world state
*/

use lawliet_types::command::Command;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        AddChargePool, CreateChannel, CreateOrgs,
    },
    helpers::{cmd_channel, get_charge_pool_mut},
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

        let channel_names: Vec<_> = eng
            .config
            .world_config
            .world_channels
            .keys()
            .copied()
            .collect();
        for name in channel_names {
            let response = Action::CreateChannel(CreateChannel { loggable: true })
                .handle(eng, ctx, actor, version, mutate)?;
            if mutate {
                let ActionResponse::CreateChannel(data) = response else {
                    unreachable!()
                };
                let channel_id = data.id;
                eng.world.world_channel_map.insert(name, channel_id);

                cmd_channel(
                    eng,
                    ctx,
                    Command::MapWorldChannel {
                        channel_id,
                        channel_name: name,
                    },
                    channel_id,
                    false,
                    None,
                );
            }
        }

        // Spawn the world's base organizations (KK, TF, SPK, …) once channels exist.
        Action::CreateOrgs(CreateOrgs {}).handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::InitializeWorld(InitializeWorldResponse {}))
    }
}
