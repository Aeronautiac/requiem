/*
* System Action
* Resolve the verdict vote: close the prosecution, announcing the outcome, and execute the
* defendant if they were found guilty.
*/

use crate::{
    action::{Action, ActionActor, ActionInterface, ActionResponse, Kill, TerminateProsecution},
    helpers::get_prosecution,
};

pub use crate::action::{ProsecutionVoteRes, ProsecutionVoteResResponse};

impl ActionInterface for ProsecutionVoteRes {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.require_system()?;

        let prosecution = get_prosecution(eng, self.prosecution_id)?;
        let prosecutor = prosecution.prosecution.prosecutor;
        let defendant = prosecution.defense.defendant;

        // Closed before the execution, not after. The verdict is a world event, and death carries
        // NoPresence — announcing afterwards would tell everyone the outcome except the person it
        // was passed on. This also releases the defendant from custody first, so they die a free
        // player rather than a prisoner.
        Action::TerminateProsecution(TerminateProsecution {
            prosecution_id: self.prosecution_id,
            verdict: Some(self.success),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        if self.success {
            Action::Kill(Kill {
                allow_link_chaining: true,
                death_message: Some(eng.config.defaults.execution_death_message.clone()),
                killer_id: Some(prosecutor),
                target_id: defendant,
                set_books_dormant: false,
                sever_links: true,
                silent: false,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::ProsecutionVoteRes(
            ProsecutionVoteResResponse {},
        ))
    }
}
