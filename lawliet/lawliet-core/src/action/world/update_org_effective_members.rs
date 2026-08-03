/*
* SYSTEM ACTION
* Broadcast every org's effective (present) member set on its channel viewport.
*
* One place recomputes which members currently count toward an org's ability requirements, instead
* of every site that could move a member's presence (a kidnap, an arrest, a death, a release). Like
* UpdateActorStatuses it diffs against the org's last_effective and says nothing when nothing moved,
* so sweeping every org on every Update is free.
*/

use smallvec::SmallVec;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    actor::ActorType,
    common::{ActorKey, Version},
    engine::Engine,
    helpers::cmd_org_effective_members,
};

pub use crate::action::{UpdateOrgEffectiveMembers, UpdateOrgEffectiveMembersResponse};

impl ActionInterface for UpdateOrgEffectiveMembers {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        _mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let orgs: SmallVec<[ActorKey; 16]> = eng
            .world
            .actors
            .iter()
            .filter_map(|(id, a)| matches!(a.actor_type, ActorType::Org(_)).then_some(id))
            .collect();

        for id in orgs {
            cmd_org_effective_members(eng, ctx, id);
        }

        Ok(ActionResponse::UpdateOrgEffectiveMembers(
            UpdateOrgEffectiveMembersResponse {},
        ))
    }
}
