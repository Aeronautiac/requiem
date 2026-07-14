/*
* SYSTEM ACTION
* Set leadership policies for an org
*/

use crate::{
    action::{
        ActionInterface, Action, ActionResponse, ChangeOrgLeader,
    },
    actor::organization::LeadershipStruct,
    helpers::get_org_mut,
};

use crate::action::ActionActor;
pub use crate::action::{SetLeadership, SetLeadershipResponse};

impl ActionInterface for SetLeadership {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let org = get_org_mut(eng, self.org_id)?;
        if mutate {
            let mut old_leader = None;
            if let Some(old_leadership) = &org.leadership_struct
                && let Some(leader) = old_leadership.leader
            {
                old_leader = Some(leader);
                // handle the case where the org previously had leadership and a leader, but leadership
                // is being set to none
                if self.policies.is_none() {
                    Action::ChangeOrgLeader(ChangeOrgLeader {
                        org_id: self.org_id,
                        new_leader: None,
                    })
                    .handle(eng, ctx, actor, version, mutate)?;
                }
            }

            let org = get_org_mut(eng, self.org_id)?;
            if let Some(policies) = self.policies {
                org.leadership_struct = Some(LeadershipStruct {
                    leader: old_leader,
                    transfer_policies: policies,
                })
            } else {
                org.leadership_struct = None;
            }
        }

        Ok(ActionResponse::SetLeadership(SetLeadershipResponse {}))
    }
}
