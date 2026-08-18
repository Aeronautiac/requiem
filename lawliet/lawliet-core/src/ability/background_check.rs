// Privately reveal the target player's true name to the ability user via a targeted
// RevealTrueName command. Like Autopsy, the command is pushed unconditionally: the
// validation pass's commands are cleared before the execution pass runs.

use lawliet_types::{
    ability::{AbilityName, BackgroundCheck},
    command::{Command, CommandRecipient},
};

use crate::{
    ability::AbilityInterface,
    helpers::{actor_id, get_channel, get_org, get_player},
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
        _version: u64,
        _mutate: bool,
    ) -> super::AbilityResult {
        let user_id = actor_id(actor).expect("expected valid actor to use BackgroundCheck");
        let true_name = get_player(eng, self.target)?.true_name.to_string();

        let recipient = if actor.is_org() {
            let org_data = get_org(eng, user_id)?;
            let viewport = get_channel(eng, org_data.channel_id)?.viewport;
            CommandRecipient::Viewport(viewport)
        } else {
            CommandRecipient::Actor(user_id)
        };

        ctx.push_cmd(
            Command::RevealTrueName {
                target_id: self.target,
                true_name,
            },
            recipient,
            eng.time,
        );

        Ok(super::AbilityStatus::Success)
    }
}
