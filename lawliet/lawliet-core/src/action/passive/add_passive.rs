/*
* SYSTEM ACTION
* Add a passive to the world
*/

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    common::PassiveKey,
    ownership::OwnershipStruct,
    passive::Passive,
    viewport::ViewportKind,
};

pub use crate::action::{AddPassive, AddPassiveResponse};

impl ActionInterface for AddPassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        _ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let id = if mutate {
            // The passive owns its viewport for its whole life; DestroyPassive frees it.
            let passive = Passive {
                ownership_struct: OwnershipStruct {
                    owner: None,
                    transferrable: self.transferrable,
                    volatile: false,
                },
                passive_type: self.passive_type,
                viewport: eng.world.add_viewport(ViewportKind::Passive),
            };
            eng.world.add_passive(passive)
        } else {
            PassiveKey::default()
        };

        Ok(ActionResponse::AddPassive(AddPassiveResponse { id }))
    }
}
