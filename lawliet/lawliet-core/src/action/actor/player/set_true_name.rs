/*
* SYSTEM ACTION
* Set (or change) a player's true name, then notify the player (and admin) of it.
*/

use lawliet_types::{action::ActionError, command::CommandRecipient};

use crate::{
    action::{ActionContext, ActionInterface, ActionResponse, ActionResult},
    command::Command,
    common::Version,
    engine::Engine,
    helpers::get_player,
};

use crate::action::ActionActor;
pub use crate::action::{SetTrueName, SetTrueNameResponse};

impl ActionInterface for SetTrueName {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        // target must be a player
        get_player(eng, self.target_id)?;

        if mutate {
            // set_player_name rejects a name held by another player.
            if !eng.world.set_player_name(self.target_id, &self.true_name) {
                return Err(ActionError::NameNotUnique);
            }
        } else if eng
            .world
            .get_player_id_by_name(&self.true_name)
            .is_some_and(|existing| existing != self.target_id)
        {
            // validate pass: mirror the mutate-path decision without mutating, so both
            // passes agree (the engine asserts validate/execute don't diverge).
            return Err(ActionError::NameNotUnique);
        }

        // Notify the player of their current true name for their personal log, and mirror it
        // to System so admin can inspect it per-user.
        for recipient in [
            CommandRecipient::Actor(self.target_id),
            CommandRecipient::System,
        ] {
            ctx.push_cmd(
                Command::TrueNameUpdate {
                    target_id: self.target_id,
                    true_name: self.true_name.clone(),
                },
                recipient,
                eng.time,
            );
        }

        Ok(ActionResponse::SetTrueName(SetTrueNameResponse {}))
    }
}
