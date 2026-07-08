use crate::{
    ability::AbilityInterface,
    action::{
        Action, ActionActor, ActionContext, ActionInterface,
        actor::player::{kill::Kill, revive::Revive, schedule_revive::ScheduleRevive},
    },
    actor::modifier::Modifier,
    command::Command,
    common::AbilityKey,
    config::ability::AbilityName,
    helpers::cmd_all_deferred,
};

pub use lawliet_types::ability::Pseudocide;

impl AbilityInterface for Pseudocide {
    fn ability_name(&self) -> crate::config::ability::AbilityName {
        AbilityName::Pseudocide
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &crate::action::ActionActor,
        _ability: AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        Action::Kill(Kill {
            allow_link_chaining: false,
            sever_links: false,
            silent: true,
            set_books_dormant: true,
            death_message: None,
            killer_id: None,
            target_id: self.target_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Action::ScheduleRevive(ScheduleRevive {
            timestamp: eng.time + eng.config.defaults.pseudocide_duration,
            revive: Revive {
                ignore_links: true,
                target_id: self.target_id,
            },
        })
        .handle(eng, ctx, actor, version, mutate)?;

        cmd_all_deferred(
            eng,
            ctx,
            Command::Death {
                target_id: self.target_id,
                true_name: self.true_name.to_lowercase(),
                death_message: self.death_message.clone(),
                role: self.role,
                notebook_transferred: self.notebook_transferred,
                ability_transferred: self.ability_transferred,
            },
            Modifier::NoPresence.into(),
            true,
        );

        Ok(())
    }
}
