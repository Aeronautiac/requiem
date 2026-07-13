/*
* SYSTEM ACTION
* Top-level engine initialization. Seeds the RNG and initializes world state.
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, InitializeWorld, SetRandomSeed,
    },
    common::Version,
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

        if eng.initialized {
            return Err(ActionError::EngineAlreadyInitialized);
        }

        Action::SetRandomSeed(SetRandomSeed { seed: self.seed })
            .handle(eng, ctx, actor, version, mutate)?;

        Action::InitializeWorld(InitializeWorld {}).handle(eng, ctx, actor, version, mutate)?;

        if mutate {
            eng.initialized = true;
        }

        Ok(ActionResponse::InitializeEngine(
            InitializeEngineResponse {},
        ))
    }
}
