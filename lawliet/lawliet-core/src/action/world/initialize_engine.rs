/*
* SYSTEM ACTION
* Top-level engine initialization. Seeds the RNG and initializes world state.
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, InitializeWorld, SetRandomSeed,
    },
    common::{Seed, Version},
    engine::Engine,
};

pub use crate::action::{InitializeEngine, InitializeEngineResponse};

impl ActionInterface for InitializeEngine {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        Action::SetRandomSeed(SetRandomSeed { seed: self.seed })
            .handle(eng, ctx, actor, version, mutate)?;

        Action::InitializeWorld(InitializeWorld {})
            .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::InitializeEngine(InitializeEngineResponse {}))
    }
}
