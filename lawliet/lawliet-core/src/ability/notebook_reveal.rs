// Player-only eye ability. Privately reveal to the user whether the target is currently
// holding a notebook (an actor's `notebooks` cache lists every notebook it currently
// HELDS). Requires the user to have at least one eye; draws the shared "shinigami eyes"
// pool (config). A negative result (target holds nothing) returns Failure and, if the
// user's eyes are volatile, costs them an eye.

use lawliet_types::{
    ability::{AbilityName, NotebookReveal},
    action::ActionError,
    command::{Command, CommandRecipient},
    passive::PassiveType,
};

use crate::{
    ability::AbilityInterface,
    helpers::{actor_get_effective_passive, actor_id, get_actor, get_player, get_player_mut},
};

impl AbilityInterface for NotebookReveal {
    fn ability_name(&self) -> AbilityName {
        AbilityName::NotebookReveal
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        _version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.player_only()?;
        let user_id = actor_id(actor).expect("expected valid actor to use NotebookReveal");

        // Eye abilities require the user to still have at least one eye.
        if get_player(eng, user_id)?.eyes < 1 {
            return Err(ActionError::NoEyes);
        }

        let holding = !get_actor(eng, self.target)?.notebooks.is_empty();

        ctx.push_cmd(
            Command::RevealNotebookHolding {
                target_id: self.target,
                holding,
            },
            CommandRecipient::Actor(user_id),
            eng.time,
        );

        if holding {
            Ok(super::AbilityStatus::Success)
        } else {
            // A negative result costs an eye when the user's eyes are volatile.
            let volatile_eyes =
                actor_get_effective_passive(eng, user_id, |p| matches!(p, PassiveType::VolatileEyes))
                    .is_some();
            if volatile_eyes && mutate {
                let user = get_player_mut(eng, user_id)?;
                user.eyes = user.eyes.saturating_sub(1);
            }
            Ok(super::AbilityStatus::Failure)
        }
    }
}
