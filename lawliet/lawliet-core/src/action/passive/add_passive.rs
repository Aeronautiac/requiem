/*
* SYSTEM ACTION
* Add a passive to the world
*/

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    common::PassiveKey,
    helpers::open_viewport,
    ownership::OwnershipStruct,
    passive::Passive,
    viewport::ViewportKind,
};

pub use crate::action::{AddPassive, AddPassiveResponse};

impl ActionInterface for AddPassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
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
                viewport: open_viewport(eng, ctx, ViewportKind::Passive),
            };
            eng.world.add_passive(passive)
        } else {
            PassiveKey::default()
        };

        Ok(ActionResponse::AddPassive(AddPassiveResponse { id }))
    }
}
