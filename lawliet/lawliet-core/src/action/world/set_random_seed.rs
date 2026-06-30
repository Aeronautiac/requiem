/*
* SYSTEM ACTION
* Set the engine RNG seed.
*/

use rand_pcg::Pcg32;
use rand_pcg::rand_core::SeedableRng;

use crate::action::{ActionInterface, ActionResponse};

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
            eng.rng_state = Pcg32::seed_from_u64(self.seed as u64);
        }
        Ok(ActionResponse::SetRandomSeed(SetRandomSeedResponse {}))
    }
}
