/*
* SYSTEM ACTION
* Set the engine RNG seed.
*/

use crate::{
    action::{
        ActionInterface, ActionResponse,
    },
    common::Seed,
};

use crate::action::ActionActor;
pub use crate::action::{SetRandomSeed, SetRandomSeedResponse};

impl ActionInterface for SetRandomSeed {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;
        if mutate {
            eng.rng_state = self.seed;
        }
        Ok(ActionResponse::SetRandomSeed(SetRandomSeedResponse {}))
    }
}
