/*
* SYSTEM ACTION
* The unified prosecution update, run from the global Update step after every action. It keeps
* everything prosecution-related consistent in one place:
*   1. cull prosecutions whose invariants broke (may terminate + emit CloseProsecution)
*   2. for every surviving prosecution: re-evaluate trial channel perms, then broadcast its
*      client-facing snapshot (and sweep the dirty set for frozen-view notices)
*
* Because Update runs after every action (only on the execute pass), this also covers state
* changes that aren't prosecution actions — e.g. a spectator gaining/losing presence re-runs the
* channel eval and the freeze sweep without any prosecution-specific trigger.
*/

use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        CullProsecutions, UpdateProsecutionChannels,
    },
    common::{ProsecutionKey, Version},
    engine::Engine,
};

use super::broadcast_prosecution;

pub use crate::action::{UpdateProsecutions, UpdateProsecutionsResponse};

impl ActionInterface for UpdateProsecutions {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Cull first so terminated prosecutions are gone before we refresh the survivors.
        Action::CullProsecutions(CullProsecutions {})
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        let ids: SmallVec<[ProsecutionKey; 8]> = eng.world.prosecutions.keys().collect();
        for prosecution_id in ids {
            Action::UpdateProsecutionChannels(UpdateProsecutionChannels { prosecution_id })
                .handle(eng, ctx, &ActionActor::System, version, mutate)?;
            broadcast_prosecution(eng, ctx, prosecution_id, mutate);
        }

        Ok(ActionResponse::UpdateProsecutions(
            UpdateProsecutionsResponse {},
        ))
    }
}
