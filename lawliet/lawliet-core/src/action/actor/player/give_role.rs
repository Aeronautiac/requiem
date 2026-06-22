/*
* SYSTEM ACTION
* Change a player's role and grant them abilities, notebooks, passives, and links associated with that role
* This operation will reset a player's role state regardless of if they already have the role
* Changing a player's role destroys any of their volatile resources
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, CreateAndGiveAbility, CreateActorLinks, PurgeVolatiles, SeverLinks, CreateAndGiveNotebook, CreateAndGivePassive, SetWorldChannelOverride, UpdateWorldChannelPerms,
    },
    actor::player::OverrideSource,
    common::ActorKey,
    config::role::Role,
    helpers::{get_player_mut, get_role_config},
};

pub use crate::action::{GiveRole, GiveRoleResponse};

impl ActionInterface for GiveRole {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let role_config = get_role_config(eng, self.role)?.clone();

        let player = get_player_mut(eng, self.target_id)?;
        if mutate {
            player.role = self.role;
            for channel_overrides in player.world_channel_overrides.values_mut() {
                channel_overrides.retain(|source, _| !matches!(source, OverrideSource::Role(_)));
            }
            player.world_channel_overrides.retain(|_, v| !v.is_empty());
        }

        Action::PurgeVolatiles(PurgeVolatiles {
            actor_id: self.target_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Action::SeverLinks(SeverLinks {
            actor_id: self.target_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        for ability in &role_config.abilities {
            Action::CreateAndGiveAbility(CreateAndGiveAbility {
                ability_name: ability.identifier.name,
                variant: ability.identifier.variant,
                transferrable: ability.transferrable,
                actor_id: self.target_id,
                volatile: true,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        for passive in &role_config.passives {
            Action::CreateAndGivePassive(CreateAndGivePassive {
                actor_id: self.target_id,
                passive_type: passive.passive_type,
                transferrable: passive.transferrable,
                volatile: true,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        for notebook in &role_config.notebooks {
            Action::CreateAndGiveNotebook(CreateAndGiveNotebook {
                fake: notebook.fake,
                volatile: true,
                actor_id: self.target_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Action::CreateActorLinks(CreateActorLinks {}).handle(eng, ctx, actor, version, mutate)?;

        for entry in &role_config.world_channel_overrides {
            Action::SetWorldChannelOverride(SetWorldChannelOverride {
                player_id: self.target_id,
                channel_name: entry.channel_name,
                source: OverrideSource::Role(self.role),
                priority: 0,
                override_data: Some(entry.override_data.clone()),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        // re-evaluate after clearing overrides (covers roles with no channel overrides)
        Action::UpdateWorldChannelPerms(UpdateWorldChannelPerms {
            player_id: self.target_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::GiveRole(GiveRoleResponse {}))
    }
}
