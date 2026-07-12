// Org ability. The org's current leader voluntarily resigns and leadership transfers per
// the org's policy (handled by the ResignLeadership action). Enforced here: if the org
// transfers by Choose, the resigning leader MUST name a successor.

use lawliet_types::{
    ability::{AbilityName, LeaderResign},
    action::ActionError,
};

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, ResignLeadership},
    actor::organization::LeadershipTransferPolicy,
    helpers::{actor_id, get_org},
};

impl AbilityInterface for LeaderResign {
    fn ability_name(&self) -> AbilityName {
        AbilityName::LeaderResign
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

        // Voluntary resign: a Choose-policy org requires the leader to name a successor.
        let org = get_org(eng, org_id)?;
        if let Some(leadership) = &org.leadership_struct
            && leadership
                .transfer_policies
                .contains(LeadershipTransferPolicy::Choose)
            && self.successor.is_none()
        {
            return Err(ActionError::MustChooseSuccessor);
        }

        Action::ResignLeadership(ResignLeadership {
            org_id,
            chosen: self.successor,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
