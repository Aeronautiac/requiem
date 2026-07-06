use lawliet_types::{
    ability::{AbilityName, Autopsy},
    action::ActionError,
    actor::State,
    command::{Command, CommandRecipient},
};

use crate::{
    ability::AbilityInterface,
    helpers::{actor_id, get_actor},
};

impl AbilityInterface for Autopsy {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::Autopsy
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        _ability: lawliet_types::common::AbilityKey,
        _version: u8,
        _mutate: bool,
    ) -> super::AbilityResult {
        let target_actor = get_actor(eng, self.target)?;
        if !target_actor.has_state(State::Dead) {
            return Err(ActionError::ActorIsAlive);
        }

        let user_id = actor_id(actor).expect("expected valid actor to use Autopsy");
        ctx.push_cmd(
            Command::RevealAutopsyMessages {
                target_id: self.target,
                range: eng.config.defaults.autopsy_window,
                redact_names: eng.config.defaults.autopsy_redaction,
            },
            CommandRecipient::Player(user_id),
            eng.time,
        );

        Ok(())
    }
}
