/*
* SYSTEM ACTION
* Add a passive to the world
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionResponse,
    },
    common::PassiveKey,
    ownership::OwnershipStruct,
    passive::Passive,
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
            let passive = Passive {
                ownership_struct: OwnershipStruct {
                    owner: None,
                    transferrable: self.transferrable,
                    volatile: false,
                },
                passive_type: self.passive_type,
            };
            eng.world.add_passive(passive)
        } else {
            PassiveKey::default()
        };

        Ok(ActionResponse::AddPassive(AddPassiveResponse { id }))
    }
}
