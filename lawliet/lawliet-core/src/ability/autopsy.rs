use lawliet_types::{
    ability::{AbilityName, Autopsy},
    action::ActionError,
    actor::State,
    command::{Command, CommandRecipient},
};

use crate::{
    ability::AbilityInterface,
    helpers::{actor_id, get_actor, get_player},
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
        _version: u64,
        _mutate: bool,
    ) -> super::AbilityResult {
        let target_actor = get_actor(eng, self.target)?;
        if !target_actor.has_state(State::Dead) {
            return Err(ActionError::ActorIsAlive);
        }

        let user_id = actor_id(actor).expect("expected valid actor to use Autopsy");
        // The target's own record, which names them as the sender of everything they said wherever
        // they said it — including anything said under a name that was not theirs.
        let log = get_player(eng, self.target)?.log;
        ctx.push_cmd(
            Command::RevealAutopsyMessages {
                log,
                range: eng.config.defaults.autopsy_window,
                redact_names: eng.config.defaults.autopsy_redaction,
            },
            CommandRecipient::Actor(user_id),
            eng.time,
        );

        Ok(super::AbilityStatus::Success)
    }
}
