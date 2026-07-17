/*
* PLAYER ACTION
* Leave a lounge
*/

use crate::{
    action::{Action, ActionInterface, ActionResponse, RemoveFromLounge},
    helpers::player_id,
};

use crate::action::ActionActor;
pub use crate::action::{LeaveLounge, LeaveLoungeResponse};

impl ActionInterface for LeaveLounge {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.player_only()?;
        let id = player_id(actor).expect("expected valid player id");

        Action::RemoveFromLounge(RemoveFromLounge {
            lounge_id: self.lounge_id,
            player_id: id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(ActionResponse::LeaveLounge(LeaveLoungeResponse {}))
    }
}
