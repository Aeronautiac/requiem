use crate::{
    ability::{AbilityInterface, AbilityResponse},
    action::ActionContext,
    actor::modifier::Modifier,
    command::Command,
    common::AbilityKey,
    config::ability::AbilityName,
    helpers::cmd_all_deferred,
};
pub use lawliet_types::ability::{AnonymousAnnouncement, AnonymousAnnouncementResponse};

impl AbilityInterface for AnonymousAnnouncement {
    fn ability_name(&self) -> crate::config::ability::AbilityName {
        AbilityName::AnonymousAnnouncement
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _: &mut ActionContext,
        _: &crate::action::ActionActor,
        _: AbilityKey,
        _: u8,
        _: bool,
    ) -> super::AbilityResult {
        cmd_all_deferred(
            eng,
            Command::AnonymousAnnouncement {
                content: self.content.clone(),
            },
            Modifier::NoPresence.into(),
        );

        Ok(AbilityResponse::AnonymousAnnouncement(
            AnonymousAnnouncementResponse {},
        ))
    }
}
