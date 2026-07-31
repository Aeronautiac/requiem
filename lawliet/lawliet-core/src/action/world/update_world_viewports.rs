/*
* SYSTEM ACTION
* Point the two world viewports at who may currently reach them.
*
* Data: every player there has ever been. Ungated — not by presence, not by death, not by
*   blackout. Nothing here is ever revoked.
* Events: every player who has presence, unless the world is under blackout, in which case
*   nobody.
*
* This is the single place either membership is decided. Both rules it applies can move for
* reasons that have nothing to do with each other — a state change, a player being created, the
* lights going out — and every one of those sites recomputes the whole answer here rather than
* working out its own delta. sync_viewport announces only genuine transitions, so recomputing when
* nothing changed costs nothing and says nothing.
*
* The data membership is recomputed here rather than granted once in AddPlayer so that "ungated"
* is a rule someone can read, instead of a property that happens to hold because no site ever
* revokes it.
*/

use indexmap::IndexSet;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    actor::{ActorType, modifier::Modifier},
    common::{ActorKey, Version},
    engine::Engine,
    helpers::sync_viewport,
};

pub use crate::action::{UpdateWorldViewports, UpdateWorldViewportsResponse};

impl ActionInterface for UpdateWorldViewports {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let players: IndexSet<ActorKey> = eng
            .world
            .actors
            .iter()
            .filter_map(|(id, actor)| {
                matches!(actor.actor_type, ActorType::Player(_)).then_some(id)
            })
            .collect();

        // Empty rather than a flag on the viewport. A blackout has to behave exactly like any
        // other loss of access — the client already knows how to hold what it was given and stop
        // treating it as current, and re-entry already backfills in order. A second mechanism
        // saying the same thing would be one more thing to keep in agreement with this one.
        let events: IndexSet<ActorKey> = if eng.world.blackout {
            IndexSet::new()
        } else {
            players
                .iter()
                .copied()
                .filter(|id| {
                    eng.world
                        .get_actor(*id)
                        .is_some_and(|a| !a.has_modifier(Modifier::NoPresence))
                })
                .collect()
        };

        let data_viewport = eng.world.data_viewport;
        sync_viewport(eng, ctx, data_viewport, players, mutate);

        let events_viewport = eng.world.events_viewport;
        sync_viewport(eng, ctx, events_viewport, events, mutate);

        Ok(ActionResponse::UpdateWorldViewports(
            UpdateWorldViewportsResponse {},
        ))
    }
}
