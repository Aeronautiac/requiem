/*
* SYSTEM ACTION
* Put a player on an organization's blacklist, or take them off it.
*
* A low-level primitive, not something an org does. Nothing an organization can decide reaches here:
* there is no ability that bars or unbars anyone, and no vote that could. It exists for the
* machinery that drives it from above — silent prosecution being the first.
*
* Blacklisting is not symmetric with unblacklisting. Barring someone KICKS them if they are
* currently a member — a bar that let a sitting member stay would not be one — while lifting the bar
* only makes them eligible again and never puts anyone back. Rejoining is AddToOrg's job, and the
* org has to decide to do it.
*
* Membership is not a precondition either way: barring somebody who was never in is the ordinary
* case, and is what stops them being invited later.
*/

use crate::{
    action::{Action, ActionInterface, ActionResponse, RemoveFromOrg},
    helpers::{get_org, get_org_mut},
};

use crate::action::ActionActor;
pub use crate::action::{SetBlacklistStatus, SetBlacklistStatusResponse};

impl ActionInterface for SetBlacklistStatus {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let org = get_org(eng, self.org_id)?;
        let was_member = org.has_member(self.actor_id);

        if mutate {
            let org = get_org_mut(eng, self.org_id)?;
            if self.blacklisted {
                org.blacklist.insert(self.actor_id);
            } else {
                org.blacklist.swap_remove(&self.actor_id);
            }
        }

        // After the bar is up, so nothing can slip back in between the two. RemoveFromOrg is the
        // one place membership is torn down — links severed, channel access dropped — and going
        // through it is what keeps a kick identical however it was triggered.
        if self.blacklisted && was_member {
            Action::RemoveFromOrg(RemoveFromOrg {
                actor_id: self.actor_id,
                org_id: self.org_id,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::SetBlacklistStatus(
            SetBlacklistStatusResponse {},
        ))
    }
}
