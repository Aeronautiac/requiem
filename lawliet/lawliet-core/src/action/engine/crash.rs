/*
* DEBUG / TESTING ACTION
* Panics the engine on purpose, so the crash path (and the UI's handling of it) can be
* exercised. Not part of normal gameplay.
*/

use crate::action::{ActionActor, ActionContext, ActionInterface, ActionResult};

pub use crate::action::Crash;

impl ActionInterface for Crash {
    fn handle(
        &mut self,
        _eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
        _actor: &ActionActor,
        _version: crate::common::Version,
        _mutate: bool,
    ) -> ActionResult {
        panic!("Manual crash triggered via the admin Crash action.");
    }
}
