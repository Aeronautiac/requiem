// Player-only eye ability. Privately reveal the target player's true name to the user,
// via the same RevealTrueName command BackgroundCheck uses. Requires the user to have at
// least one eye; draws the shared "shinigami eyes" pool (config).

use lawliet_types::{
    ability::{AbilityName, TrueNameReveal},
    action::ActionError,
    command::{Command, CommandRecipient},
};

use crate::{
    ability::AbilityInterface,
    helpers::{actor_id, get_player},
};

impl AbilityInterface for TrueNameReveal {
    fn ability_name(&self) -> AbilityName {
        AbilityName::TrueNameReveal
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        _version: u8,
        _mutate: bool,
    ) -> super::AbilityResult {
        actor.player_only()?;
        let user_id = actor_id(actor).expect("expected valid actor to use TrueNameReveal");

        // Eye abilities require the user to still have at least one eye.
        if get_player(eng, user_id)?.eyes < 1 {
            return Err(ActionError::NoEyes);
        }

        let true_name = get_player(eng, self.target)?.true_name.to_string();
        ctx.push_cmd(
            Command::RevealTrueName {
                target_id: self.target,
                true_name,
            },
            CommandRecipient::Actor(user_id),
            eng.time,
        );

        Ok(super::AbilityStatus::Success)
    }
}
