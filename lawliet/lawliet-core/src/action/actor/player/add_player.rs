/*
* SYSTEM ACTION
* Add a new player to the world
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, AddChargePool, AddToWorldChannels, CreateAndGiveAbility, GiveRole,
        SetTrueName,
    },
    command::Command,
    common::{ActorKey, Version},
    engine::Engine,
    helpers::{cmd_world_event, get_actor_mut, get_charge_pool_mut, sync_presence},
};

// true names must be unique

pub use crate::action::{AddPlayer, AddPlayerResponse};

impl ActionInterface for AddPlayer {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        if eng.world.get_player_id_by_name(&self.true_name).is_some() {
            return Err(ActionError::NameNotUnique);
        }

        let player_id = if mutate {
            eng.world
                .add_player(&self.true_name, self.starting_role)
                .unwrap()
        } else {
            ActorKey::default()
        };

        // player will only be physically created in the mutation path
        if mutate {
            // A new player starts present, so they enter the presence viewport here — and
            // because their watermark starts at zero, entry hands them every world event since
            // the game began. That is what the BasePlayer stream existed for ("any player shall
            // be fed these commands, even if created AFTER the command was sent"), and it now
            // falls out of ordinary backfill with no separate stream and no born-position
            // bookkeeping on the server.
            sync_presence(eng, ctx, mutate);

            // Announce the slot, AFTER the entry above so this player's own backfill of the
            // presence viewport hands them every earlier MapPlayer — i.e. the existing roster —
            // before their own arrival goes out to everyone else.
            cmd_world_event(eng, ctx, Command::MapPlayer { player_id });

            // add pools BEFORE giving abilities (the pools must exist beforehand)
            let pools = eng.config.player_config.charge_pools.clone();
            for (name, specifier) in pools {
                let pool_response = Action::AddChargePool(AddChargePool {
                    base_charges: specifier.charges,
                    base_reset_time: specifier.reset_time,
                })
                .handle(eng, ctx, actor, version, mutate)?;
                let ActionResponse::AddChargePool(data) = pool_response else {
                    unreachable!()
                };
                let pool = get_charge_pool_mut(eng, data.id)?;
                pool.on_link();
                let player_actor = get_actor_mut(eng, player_id)?;
                player_actor.pool_map.insert(name, data.id);
            }

            let default_abilities = eng.config.defaults.universal_abilities.clone();
            for default_ability in default_abilities {
                Action::CreateAndGiveAbility(CreateAndGiveAbility {
                    ability_name: default_ability.name,
                    transferrable: false,
                    variant: default_ability.variant,
                    actor_id: player_id,
                    volatile: false,
                })
                .handle(eng, ctx, actor, version, mutate)?;
            }

            Action::AddToWorldChannels(AddToWorldChannels { player_id })
                .handle(eng, ctx, actor, version, mutate)?;

            Action::GiveRole(GiveRole {
                target_id: player_id,
                role: self.starting_role,
            })
            .handle(eng, ctx, actor, version, mutate)?;

            // Emit the initial true-name notification (the name itself is already set by
            // world.add_player above; this re-affirms it and notifies the player + admin).
            Action::SetTrueName(SetTrueName {
                target_id: player_id,
                true_name: self.true_name.clone(),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::AddPlayer(AddPlayerResponse {
            id: player_id,
        }))
    }
}
