// Grant the IPP state to the target player. The state's modifiers (strengthened
// presence + write immunity) come from engine config, applied by AddState.

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, AddState},
    actor::state::State,
    config::ability::AbilityName,
    helpers::get_player,
};

pub use lawliet_types::ability::Ipp;

impl AbilityInterface for Ipp {
    fn ability_name(&self) -> AbilityName {
        AbilityName::Ipp
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        _actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        // No actor gate: this ability only writes a state to the target and never
        // touches channel membership, so it doesn't matter whether a player or an org
        // uses it.

        // The target must be a real player before we grant it a state.
        get_player(eng, self.target)?;

        Action::AddState(AddState {
            actor_id: self.target,
            state: State::Ipp,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
