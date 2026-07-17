/*
* SYSTEM ACTION
* The org's current leader resigns. The successor is picked per the org's
* LeadershipTransferPolicy:
* - Choose: the resigning leader's named successor (`chosen`), if it's a present member.
* - Random: an rng-picked present member.
* If neither policy yields a successor (no candidates, or Choose with no valid pick and
* no Random policy), the seat is vacated (leader = None).
*
* This action is general (voluntary + future involuntary transfers). The requirement
* that a Choose-policy org's leader actually name a successor is enforced by the
* LeaderResign ability, not here.
*/

use rand_pcg::rand_core::Rng;
use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionError, ActionInterface, ActionResponse, ActionResult,
        ChangeOrgLeader,
    },
    actor::{modifier::Modifier, organization::LeadershipTransferPolicy},
    common::{ActorKey, Version},
    engine::Engine,
    helpers::{get_actor, get_org},
};

pub use crate::action::{ResignLeadership, ResignLeadershipResponse};

impl ActionInterface for ResignLeadership {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let org = get_org(eng, self.org_id)?;
        let Some(leadership) = &org.leadership_struct else {
            return Err(ActionError::OrgDoesntHaveLeadership);
        };
        let Some(current_leader) = leadership.leader else {
            return Err(ActionError::OrgDoesntHaveLeadership);
        };
        let policies = leadership.transfer_policies;

        // A named successor must be valid whether or not it ends up used: a present member
        // of this org, and not the resigning leader.
        if let Some(chosen) = self.chosen {
            if chosen == current_leader {
                return Err(ActionError::AlreadyLeader);
            }
            if !org.has_member(chosen) {
                return Err(ActionError::PlayerNotInOrg);
            }
            if get_actor(eng, chosen)?.has_modifier(Modifier::NoPresence) {
                return Err(ActionError::UserNotPresent);
            }
        }

        // Present members eligible to inherit leadership (the resigning leader excluded).
        let candidates: SmallVec<[ActorKey; 8]> = org
            .members
            .keys()
            .copied()
            .filter(|id| *id != current_leader)
            .filter(|id| get_actor(eng, *id).is_ok_and(|a| !a.has_modifier(Modifier::NoPresence)))
            .collect();

        // chosen (if any) is validated above, so a Choose policy can use it directly.
        let new_leader = if policies.contains(LeadershipTransferPolicy::Choose)
            && let Some(chosen) = self.chosen
        {
            Some(chosen)
        } else if policies.contains(LeadershipTransferPolicy::Random) && !candidates.is_empty() {
            if mutate {
                let idx = (eng.rng_state.next_u32() as usize) % candidates.len();
                Some(candidates[idx])
            } else {
                // Don't advance the RNG on the validation pass; a valid placeholder member
                // keeps ChangeOrgLeader's membership check satisfied without desyncing.
                Some(candidates[0])
            }
        } else {
            None
        };

        Action::ChangeOrgLeader(ChangeOrgLeader {
            org_id: self.org_id,
            new_leader,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::ResignLeadership(
            ResignLeadershipResponse {},
        ))
    }
}
