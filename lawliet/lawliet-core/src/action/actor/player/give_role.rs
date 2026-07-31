/*
* SYSTEM ACTION
* Change a player's role and grant them abilities, notebooks, passives, and links associated with that role
* This operation will reset a player's role state regardless of if they already have the role
* Changing a player's role destroys any of their volatile resources
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        CreateActorLinks, CreateAndGiveAbility, CreateAndGiveNotebook, CreateAndGivePassive,
        CreateAndGiveProfile, PurgeVolatiles, RemoveFromChannel, SeverLinks,
    },
    actor::ActorDisplay,
    channel::BlueprintDisplayKind,
    command::Command,
    helpers::{get_player, get_player_mut, get_role_config, get_world_channel_id},
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

        // Read before the role is overwritten: the rooms they are in for being what they were have
        // to be given up before they can be given the new ones.
        let previous_seats = get_role_config(eng, get_player(eng, self.target_id)?.role)?
            .world_channel_profiles
            .clone();

        let player = get_player_mut(eng, self.target_id)?;
        if mutate {
            player.role = self.role;
        }

        for seat in &previous_seats {
            let Ok(channel_id) = get_world_channel_id(eng, seat.channel_name) else {
                continue;
            };
            Action::RemoveFromChannel(RemoveFromChannel {
                channel_id,
                player_id: self.target_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
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

        // The world channels this role is in for being what it is. A channel with no blueprint of
        // its own gets its guest list from here.
        for seat in &role_config.world_channel_profiles {
            let channel_id = get_world_channel_id(eng, seat.channel_name)?;
            Action::CreateAndGiveProfile(CreateAndGiveProfile {
                channel_id,
                player_id: self.target_id,
                display: match seat.blueprint.display_kind {
                    BlueprintDisplayKind::OwnerRaw => ActorDisplay::Raw(self.target_id),
                },
                visible: seat.blueprint.start_visible,
                shared: false,
                transferrable: false,
                perm_policy: seat.blueprint.perm_policy,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        // Notify the player of their (new) role for their personal log, and mirror it to
        // System so admin can inspect any player's role.
        for recipient in [
            CommandRecipient::Actor(self.target_id),
            CommandRecipient::System,
        ] {
            ctx.push_cmd(
                Command::RoleUpdate {
                    target_id: self.target_id,
                    role: self.role,
                },
                recipient,
                eng.time,
            );
        }

        Ok(ActionResponse::GiveRole(GiveRoleResponse {}))
    }
}
