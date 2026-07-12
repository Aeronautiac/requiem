// Privately reveal the target player's true name to the ability user via a targeted
// RevealTrueName command. Like Autopsy, the command is pushed unconditionally: the
// validation pass's commands are cleared before the execution pass runs.

use lawliet_types::{
    ability::{AbilityName, BackgroundCheck},
    command::{Command, CommandRecipient},
};

use crate::{
    ability::AbilityInterface,
    helpers::{actor_id, get_player},
};

impl AbilityInterface for BackgroundCheck {
    fn ability_name(&self) -> AbilityName {
        AbilityName::BackgroundCheck
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
        let true_name = get_player(eng, self.target)?.true_name.to_string();
        let user_id = actor_id(actor).expect("expected valid actor to use BackgroundCheck");

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
