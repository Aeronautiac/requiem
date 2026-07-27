/*
* Authoritative Action
* Recompute every passive viewport's membership from scratch.
*
* Access to a passive's accumulated log is gated on EFFECTIVE possession, not ownership: an actor
* linked to another by ActorLinkType::Passive inherits their passives, so Watari holding the log
* means L reads it too — until Watari picks up DisablePassiveLinks, at which point L stops.
*
* Should be called when:
* - a passive is created, given, or destroyed
* - an actor link is created or severed
* - an actor's state changes (DisablePassiveLinks rides on states)
*
* Like UpdateBugVisibilities this is a full recompute; sync_viewport reduces it to the handful of
* commands that represent real access changes, so calling it broadly costs little.
*/

use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::{
    ActorKey,
    action::{ActionActor, ActionInterface, ActionResponse},
    actor::ActorType,
    common::ViewportKey,
    helpers::{actor_reaches_passive, sync_viewport},
};

pub use crate::action::{UpdatePassiveVisibilities, UpdatePassiveVisibilitiesResponse};

impl ActionInterface for UpdatePassiveVisibilities {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let players: SmallVec<[ActorKey; 16]> = eng
            .world
            .actors
            .iter()
            .filter_map(|(id, a)| matches!(a.actor_type, ActorType::Player(_)).then_some(id))
            .collect();

        // Resolve every passive before applying any of it: sync_viewport needs &mut Engine, and the
        // iteration is borrowing the passive map.
        let passives: SmallVec<[(ViewportKey, IndexSet<ActorKey>); 16]> = eng
            .world
            .passives
            .iter()
            .map(|(passive_id, passive)| (passive_id, passive.viewport))
            .collect::<SmallVec<[_; 16]>>()
            .into_iter()
            .map(|(passive_id, viewport)| {
                let readers = players
                    .iter()
                    .copied()
                    .filter(|player| actor_reaches_passive(eng, *player, passive_id))
                    .collect();
                (viewport, readers)
            })
            .collect();

        for (viewport, readers) in passives {
            sync_viewport(eng, ctx, viewport, readers, mutate);
        }

        Ok(ActionResponse::UpdatePassiveVisibilities(
            UpdatePassiveVisibilitiesResponse {},
        ))
    }
}
