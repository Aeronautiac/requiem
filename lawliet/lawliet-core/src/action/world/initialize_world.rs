/*
* SYSTEM ACTION
* Initialize any necessary world state
*/

use lawliet_types::command::{Command, CommandRecipient};
use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        AddAbility, AddChargePool, AddPassive, CreateChannel, CreateOrgs,
    },
    channel::ChannelKind,
    common::ViewportKey,
    helpers::{cmd_channel, get_charge_pool_mut},
    viewport::ViewportKind,
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

        // The viewports no action opens: they come into existence with the world and outlive
        // everything in it, so they are announced here instead. Pushed before anything else so
        // each heads its own viewport's history, exactly as open_viewport arranges for the rest.
        // The three contact-log records are among them.
        let world_viewports: SmallVec<[(ViewportKey, ViewportKind); 8]> = [
            (eng.world.events_viewport, ViewportKind::WorldEvents),
            (eng.world.data_viewport, ViewportKind::WorldData),
        ]
        .into_iter()
        .chain(
            eng.world
                .contact_log_viewports()
                .map(|(_, viewport)| (viewport, ViewportKind::ContactLog)),
        )
        .collect();
        for (viewport, kind) in world_viewports {
            ctx.push_cmd(
                Command::MapViewport { viewport, kind },
                CommandRecipient::Viewport(viewport),
                eng.time,
            );
        }

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

        // world channel creation
        let channels: Vec<_> = eng
            .config
            .world_config
            .world_channels
            .iter()
            .map(|(name, blueprint)| (*name, *blueprint))
            .collect();
        for (name, base_profile) in channels {
            let response = Action::CreateChannel(CreateChannel {
                loggable: true,
                base_profile,
            })
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
                    Command::MapChannel {
                        channel_id,
                        kind: ChannelKind::World(name),
                    },
                    channel_id,
                    false,
                    None,
                );
            }
        }

        // The news anchor's kit: created ownerless here, its ownership handed to whoever a host names
        // as anchor (SetNewsAnchor). Non-transferrable so a killer never inherits it, non-volatile so
        // nothing purges it — it waits on the last anchor until it is handed on. Held on the world so
        // it can be reassigned rather than remade, which is what keeps its charges across a change of
        // anchor.
        let anchor_abilities = eng.config.world_config.news_anchor_abilities.clone();
        for identifier in anchor_abilities {
            let response = Action::AddAbility(AddAbility {
                ability_name: identifier.name,
                variant: identifier.variant,
                transferrable: false,
            })
            .handle(eng, ctx, actor, version, mutate)?;
            if mutate {
                let ActionResponse::AddAbility(data) = response else {
                    unreachable!()
                };
                eng.world.news.anchor_abilities.insert(data.id);
            }
        }

        let anchor_passives = eng.config.world_config.news_anchor_passives.clone();
        for passive_type in anchor_passives {
            let response = Action::AddPassive(AddPassive {
                passive_type,
                transferrable: false,
            })
            .handle(eng, ctx, actor, version, mutate)?;
            if mutate {
                let ActionResponse::AddPassive(data) = response else {
                    unreachable!()
                };
                eng.world.news.anchor_passives.insert(data.id);
            }
        }

        // Spawn the world's base organizations (KK, TF, SPK, …) once channels exist.
        Action::CreateOrgs(CreateOrgs {}).handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::InitializeWorld(InitializeWorldResponse {}))
    }
}
