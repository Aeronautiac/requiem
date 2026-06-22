/*
* System Action
*/

use crate::{
    ProsecutionKey,
    action::{
        ActionInterface, Action, ActionActor, ActionResponse, Kill, TerminateProsecution,
    },
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

        if self.success {
            Action::Kill(Kill {
                allow_link_chaining: true,
                death_message: Some(eng.config.defaults.execution_death_message.clone()),
                killer_id: Some(prosecution.prosecutor),
                target_id: prosecution.defense.defendant,
                set_books_dormant: false,
                sever_links: true,
                silent: false,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

            // TODO:
            // success announcement (deferred)
        } else {
            // TODO:
            // deferred failure announcement
        }

        Action::TerminateProsecution(TerminateProsecution {
            prosecution_id: self.prosecution_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(ActionResponse::ProsecutionVoteRes(
            ProsecutionVoteResResponse {},
        ))
    }
}
