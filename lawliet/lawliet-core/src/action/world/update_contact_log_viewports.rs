/*
* Authoritative Action
* Recompute the three world contact-log viewports' membership from scratch.
*
* Access to a contact log is gated on EFFECTIVE possession of the matching ContactLogs passive, not
* ownership: an actor linked to another by ActorLinkType::Passive inherits their passives, so Watari
* holding the log means L reads it too — until Watari picks up DisablePassiveLinks, at which point L
* stops. The record itself is a world singleton (World::contact_log_viewports); this only decides
* who is entered into it, and entering backfills the whole history — which is what lets a passive
* granted late still see everything logged before it existed.
*
* Should be called when:
* - a passive is created, given, taken, or destroyed
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
    common::{PassiveKey, ViewportKey},
    helpers::{actor_reaches_passive, sync_viewport},
    passive::{ContactLogType, PassiveType},
};

pub use crate::action::{UpdateContactLogViewports, UpdateContactLogViewportsResponse};

impl ActionInterface for UpdateContactLogViewports {
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

        // The contact-log passives, paired with which record each feeds. A given type can have more
        // than one passive (two players each holding an Even log), and reaching ANY of a type's
        // passives is what puts a player in that type's viewport.
        let log_passives: SmallVec<[(PassiveKey, ContactLogType); 8]> = eng
            .world
            .passives
            .iter()
            .filter_map(|(id, p)| match p.passive_type {
                PassiveType::ContactLogs(kind) => Some((id, kind)),
                _ => None,
            })
            .collect();

        // Resolve each viewport's membership before applying any of it: sync_viewport needs
        // &mut Engine, and the reachability checks are borrowing the world.
        let memberships: SmallVec<[(ViewportKey, IndexSet<ActorKey>); 3]> = eng
            .world
            .contact_log_viewports()
            .map(|(kind, viewport)| {
                let readers = players
                    .iter()
                    .copied()
                    .filter(|player| {
                        log_passives.iter().any(|(passive_id, passive_kind)| {
                            *passive_kind == kind
                                && actor_reaches_passive(eng, *player, *passive_id)
                        })
                    })
                    .collect();
                (viewport, readers)
            })
            .collect();

        for (viewport, readers) in memberships {
            sync_viewport(eng, ctx, viewport, readers, mutate);
        }

        Ok(ActionResponse::UpdateContactLogViewports(
            UpdateContactLogViewportsResponse {},
        ))
    }
}
