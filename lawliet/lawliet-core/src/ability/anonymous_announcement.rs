use crate::{
    ability::AbilityInterface, action::ActionContext, actor::modifier::Modifier, command::Command,
    common::AbilityKey, config::ability::AbilityName, helpers::cmd_all_deferred,
};
pub use lawliet_types::ability::AnonymousAnnouncement;

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
        mutate: bool,
    ) -> super::AbilityResult {
        cmd_all_deferred(
            eng,
            ctx,
            Command::AnonymousAnnouncement {
                content: self.content.clone(),
            },
            Modifier::NoPresence.into(),
            true,
            true,
            mutate,
        );

        Ok(super::AbilityStatus::Success)
    }
}
