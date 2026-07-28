/*
* SYSTEM ACTION
* Change whether an existing member of an organization is an OG.
*
* AddToOrg can only say what somebody was at the moment they joined. This is the only way the flag
* moves afterwards — an org recognising a later arrival as one of its own, or taking that back.
*
* Membership is a precondition, not something this creates: OG is a property OF a membership, and
* setting it on a non-member would leave a flag with nothing to hang on.
*/

use crate::{
    action::{ActionError, ActionInterface, ActionResponse},
    helpers::{cmd_og_status, get_org_mut},
};

use crate::action::ActionActor;
pub use crate::action::{SetOgStatus, SetOgStatusResponse};

impl ActionInterface for SetOgStatus {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let org = get_org_mut(eng, self.org_id)?;
        let member = org
            .members
            .get_mut(&self.actor_id)
            .ok_or(ActionError::PlayerNotInOrg)?;

        // Read before the write; afterwards there is nothing to compare against. Setting the flag
        // to what it already is changes nothing and saying so again is noise.
        let changed = member.og != self.og;
        if mutate {
            member.og = self.og;
        }

        if changed {
            cmd_og_status(eng, ctx, self.org_id, self.actor_id, self.og);
        }

        Ok(ActionResponse::SetOgStatus(SetOgStatusResponse {}))
    }
}
