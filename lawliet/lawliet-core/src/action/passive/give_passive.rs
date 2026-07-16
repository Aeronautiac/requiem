/*
* SYSTEM ACTION
* Transfer ownership of an ability to a specified actor and then reset links
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, ActionActor, ActionError, ActionResponse,
    },
    command::Command,
    helpers::{get_actor, get_actor_mut, get_passive, get_passive_mut},
};

pub use crate::action::{GivePassive, GivePassiveResponse};

impl ActionInterface for GivePassive {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        get_actor(eng, self.actor_id)?;

        let passive = get_passive(eng, self.passive_id)?;
        let passive_type = passive.passive_type;
        let old_owner = passive.ownership_struct.owner;
        if let Some(owner) = old_owner {
            if owner == self.actor_id {
                return Err(ActionError::ItemAlreadyOwned);
            }
            if mutate {
                let other_actor = get_actor_mut(eng, owner).unwrap(); // should
                // crash if the owner is an actor that doesnt exist (the state is invalid)
                other_actor.remove_passive(self.passive_id);
            }
        }

        let passive = get_passive_mut(eng, self.passive_id)?;
        if mutate {
            passive
                .ownership_struct
                .set_owner(self.actor_id, self.volatile);
            let actor_data = get_actor_mut(eng, self.actor_id)?;
            actor_data.add_passive(self.passive_id);

            // Drop the passive from the previous owner's observable list, then reveal it to the
            // new owner.
            if let Some(owner) = old_owner {
                ctx.push_cmd(
                    Command::RemovePassive {
                        passive_id: self.passive_id,
                    },
                    CommandRecipient::Actor(owner),
                    eng.time,
                );
            }
            ctx.push_cmd(
                Command::UpdatePassiveView {
                    passive_type,
                    passive_id: self.passive_id,
                    owner_id: self.actor_id,
                },
                CommandRecipient::Actor(self.actor_id),
                eng.time,
            );
        }

        Ok(ActionResponse::GivePassive(GivePassiveResponse {}))
    }
}
