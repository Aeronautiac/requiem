use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionContext, ActionInterface, actor::player::kill::Kill},
    common::AbilityKey,
    config::ability::AbilityName,
    helpers::player_id,
};

pub use lawliet_types::ability::Gun;

impl AbilityInterface for Gun {
    fn ability_name(&self) -> crate::config::ability::AbilityName {
        AbilityName::Gun
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &crate::action::ActionActor,
        _: AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        let id = player_id(actor);

        Action::Kill(Kill{
            allow_link_chaining: true,
            sever_links: true,
            silent: false,
            set_books_dormant: false,
            death_message: Some("They were found dead with 3 gunshot wounds to the back of the head. Their death was ruled a suicide.".into()),
            killer_id: id,
            target_id: self.target_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
