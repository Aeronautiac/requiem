/*
* SYSTEM ACTION
* Handle a poll timeout
* (try to resolve the poll, if it accepts, execute, else clean it up)
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, PollCleanup, ActionExt,
    },
    common::{PollKey, Version},
    engine::Engine,
    helpers::get_poll,
    poll::PolicyResult,
};

pub use crate::action::{PollTimeout, PollTimeoutResponse};

impl ActionInterface for PollTimeout {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let poll = get_poll(eng, self.poll_id)?;
        let mut acc_payload = poll.accept_payload.clone();
        let mut rej_payload = poll.reject_payload.clone();
        let policy_res = poll.timeout_policy(eng);

        if acc_payload.as_mut().is_some_and(|p| p.validate(eng, ctx, actor, version).is_err())
            || rej_payload.as_mut().is_some_and(|p| p.validate(eng, ctx, actor, version).is_err())
        {
            // TODO: tell frontend to acknowledge action failure
        } else {
            match policy_res {
                PolicyResult::Accept => {
                    if let Some(mut act) = acc_payload {
                        act.handle(eng, ctx, actor, version, mutate)?;
                    }
                }
                PolicyResult::Reject => {
                    if let Some(mut act) = rej_payload {
                        act.handle(eng, ctx, actor, version, mutate)?;
                    }
                }
                PolicyResult::Inconclusive => {
                    // TODO: tell frontend to acknowledge inconclusive result
                }
            }
        }

        Action::PollCleanup(PollCleanup {
            poll_id: self.poll_id,
            cancelled: false,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::PollTimeout(PollTimeoutResponse {}))
    }
}
