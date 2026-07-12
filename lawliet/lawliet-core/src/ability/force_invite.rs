// Org ability. Add a player into the acting org immediately, with no true-name guess.
// (A blacklist check will eventually live in AddToOrg; nothing extra is needed here.)

use lawliet_types::ability::{AbilityName, ForceInvite};

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, AddToOrg},
    helpers::{actor_id, get_player},
};

impl AbilityInterface for ForceInvite {
    fn ability_name(&self) -> AbilityName {
        AbilityName::ForceInvite
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.org_only()?;
        let org_id = actor_id(actor).expect("org actor has an id");

        // Validate the target is a real player before handing it to AddToOrg.
        get_player(eng, self.target)?;

        Action::AddToOrg(AddToOrg {
            leader: false,
            og: false,
            actor_id: self.target,
            org_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
