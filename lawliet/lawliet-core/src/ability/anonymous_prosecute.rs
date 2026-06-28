use lawliet_types::{
    ability::{AbilityName, AnonymousProsecute},
    action::{Action, ActionActor, StartProsecution},
    actor::ActorDisplay,
    prosecution::ProsecutionSource,
};

use crate::{
    ability::AbilityInterface,
    action::ActionInterface,
    helpers::{actor_id, get_player},
};

impl AbilityInterface for AnonymousProsecute {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::AnonymousProsecute
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        // TODO:
        // add non-autonomous prosecutions

        // TODO:
        // potentially display the org rather than the role if its somehow an org, but for now its
        // only players
        let prosecutor_id =
            actor_id(actor).expect("expected valid actor to have anonymous prosecute ability");
        let prosecutor_data = get_player(eng, prosecutor_id)?;

        Action::StartProsecution(StartProsecution {
            autonomous: true,
            defendant_id: self.target,
            source: ProsecutionSource::Ability(ability),
            defendant_display: ActorDisplay::Raw(self.target),
            prosecutor_display: ActorDisplay::Role(prosecutor_data.role),
            prosecutor_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(())
    }
}
