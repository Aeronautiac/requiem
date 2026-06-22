/*
* SYSTEM ACTION
* Remove a player from an organization
*/

use crate::{
    action::{
        ActionInterface, Action, ActionError, ActionResponse, UpdateKidnapChannels,
    },
    actor::{ActorLink, ActorLinkType},
    common::ActorKey,
    helpers::{get_actor, get_actor_mut, get_org_mut},
};

use crate::action::ActionActor;
pub use crate::action::{RemoveFromOrg, RemoveFromOrgResponse};

impl ActionInterface for RemoveFromOrg {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        get_actor(eng, self.actor_id)?;

        let org = get_org_mut(eng, self.org_id)?;
        if !org.has_member(self.actor_id) {
            return Err(ActionError::PlayerNotInOrg);
        }

        if mutate {
            org.remove_member(self.actor_id);
            let actor = get_actor_mut(eng, self.actor_id)?;
            actor.sever_link(ActorLink {
                link_type: ActorLinkType::Passive,
                link_dest: self.org_id,
            });
            dbg!(&actor);
        }

        Action::UpdateKidnapChannels(UpdateKidnapChannels {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::RemoveFromOrg(RemoveFromOrgResponse {}))
    }
}
