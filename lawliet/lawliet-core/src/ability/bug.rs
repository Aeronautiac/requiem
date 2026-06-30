use lawliet_types::{
    ability::{AbilityName, Bug},
    action::{Action, ActionActor, CreateBug},
    bug::BugSource,
};

use crate::{ability::AbilityInterface, action::ActionInterface};

impl AbilityInterface for Bug {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::Bug
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        _actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        Action::CreateBug(CreateBug {
            target_id: self.target,
            source: BugSource::Ability(ability),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(())
    }
}
