// Org ability. Delegate a prosecution: invite `invitee` into the acting org, then start
// a prosecution with the invited player as the prosecutor against `defendant`. Draws the
// org's Invite and Prosecution pools.

use lawliet_types::{
    ability::{AbilityName, Outsource},
    actor::ActorDisplay,
    prosecution::ProsecutionSource,
};

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, AddToOrg, StartProsecution},
    helpers::actor_id,
};

impl AbilityInterface for Outsource {
    fn ability_name(&self) -> AbilityName {
        AbilityName::Outsource
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        ability: crate::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.org_only()?;
        let org_id = actor_id(actor).expect("org actor has an id");

        // Bring the prosecutor into the org first.
        Action::AddToOrg(AddToOrg {
            leader: false,
            og: false,
            actor_id: self.invitee,
            org_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        // Then file the prosecution under the invited player's identity.
        Action::StartProsecution(StartProsecution {
            autonomous: eng.config.defaults.prosecution_autonomous,
            defendant_id: self.defendant,
            source: ProsecutionSource::Ability(ability),
            defendant_display: ActorDisplay::Raw(self.defendant),
            prosecutor_display: ActorDisplay::Raw(self.invitee),
            prosecutor_id: self.invitee,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
