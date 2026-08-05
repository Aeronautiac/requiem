use crate::{
    ability::AbilityInterface, action::ActionContext, common::AbilityKey,
    config::ability::AbilityName, helpers::cmd_world_event,
};
pub use lawliet_types::ability::AnonymousAnnouncement;
use lawliet_types::command::Command;

impl AbilityInterface for AnonymousAnnouncement {
    fn ability_name(&self) -> crate::config::ability::AbilityName {
        AbilityName::AnonymousAnnouncement
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        _: &crate::action::ActionActor,
        _: AbilityKey,
        _: u8,
        _mutate: bool,
    ) -> super::AbilityResult {
        cmd_world_event(
            eng,
            ctx,
            Command::AnonymousAnnouncement {
                content: self.content.clone(),
            },
        );

        Ok(super::AbilityStatus::Success)
    }
}
