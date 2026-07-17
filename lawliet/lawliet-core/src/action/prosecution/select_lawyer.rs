/*
* PLAYER ACTION (defendant only)
* Select a lawyer during the custody phase.
*
* Preconditions:
* - caller is the defendant in this prosecution
* - prosecution is in the Custody phase
* - lawyer target exists, has presence, and is not the defendant
* - no lawyer has already been selected
*
* On execution:
* - create a private channel between defendant and lawyer
* - set defense.lawyer
*
* TODO: commands
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, CreateChannel,
    },
    actor::modifier::Modifier,
    common::Version,
    engine::Engine,
    helpers::{get_actor, get_prosecution_mut, player_id, require_player},
    prosecution::{Lawyer, ProsecutionPhase},
};

pub use crate::action::{SelectLawyer, SelectLawyerResponse};

impl ActionInterface for SelectLawyer {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.player_only()?;
        let caller = player_id(actor).expect("already validated as player");

        let prosecution = get_prosecution_mut(eng, self.prosecution_id)?;

        if prosecution.defense.defendant != caller {
            return Err(ActionError::NotInProsecution);
        }
        if prosecution.defense.lawyer.is_some() {
            return Err(ActionError::LawyerAlreadySelected);
        }
        if !matches!(prosecution.phase, ProsecutionPhase::Custody { .. }) {
            return Err(ActionError::NotACustodyPhase);
        }
        if self.lawyer_id == caller {
            return Err(ActionError::CannotBeOwnLawyer);
        }

        require_player(eng, self.lawyer_id)?;
        if get_actor(eng, self.lawyer_id)
            .expect("already validated")
            .has_modifier(Modifier::NoPresence)
        {
            return Err(ActionError::UserNotPresent);
        }

        let channel_response = Action::CreateChannel(CreateChannel { loggable: false }).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;
        let ActionResponse::CreateChannel(channel_data) = channel_response else {
            unreachable!()
        };

        if mutate {
            get_prosecution_mut(eng, self.prosecution_id)
                .expect("already validated")
                .defense
                .lawyer = Some(Lawyer {
                actor_id: self.lawyer_id,
                channel_id: channel_data.id,
            });
        }

        Ok(ActionResponse::SelectLawyer(SelectLawyerResponse {}))
    }
}
