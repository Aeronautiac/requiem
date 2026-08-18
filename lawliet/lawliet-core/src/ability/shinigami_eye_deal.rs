// Take the shinigami eye deal: the ability consumes itself and grants the user the TrueNameReveal
// ability, and the world is told that someone of the user's role made the deal — never which player.
// Player-only; aliveness is enforced by the general UseAbility gate.

use lawliet_types::{ability::AbilityName, actor::ActorDisplay, command::Command};

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, CreateAndGiveAbility, DestroyAbility},
    common::AbilityKey,
    helpers::{actor_id, cmd_world_event, get_player},
};

pub use lawliet_types::ability::ShinigamiEyeDeal;

impl AbilityInterface for ShinigamiEyeDeal {
    fn ability_name(&self) -> AbilityName {
        AbilityName::ShinigamiEyeDeal
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        ability: AbilityKey,
        version: u64,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.player_only()?;
        let user_id = actor_id(actor).expect("expected valid actor to use ShinigamiEyeDeal");
        let role = get_player(eng, user_id)?.role;

        Action::CreateAndGiveAbility(CreateAndGiveAbility {
            ability_name: AbilityName::TrueNameReveal,
            variant: 0,
            transferrable: false,
            actor_id: user_id,
            volatile: false,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        cmd_world_event(
            eng,
            ctx,
            Command::EyeDealTaken {
                user: ActorDisplay::Role(role),
            },
        );

        // Consume this ability: DestroyAbility removes it from the world and, since it is owned,
        // hides it from the user's client.
        Action::DestroyAbility(DestroyAbility {
            ability_id: ability,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
