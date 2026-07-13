/*
* SYSTEM ACTION
* Handle a poll timeout
* (try to resolve the poll, if it accepts, execute, else clean it up)
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, PollCleanup, ActionExt,
    },
    common::Version,
    engine::Engine,
    helpers::get_poll,
    poll::{PollOutcome, PolicyResult},
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

        // Determine how the poll ended (this is what the frontend is told via ClosePoll).
        // A payload that no longer validates cancels the poll instead of resolving it.
        // Decide the outcome and which payload (if any) to run. A payload that no longer
        // validates cancels the poll instead of resolving it.
        let invalid = acc_payload.as_mut().is_some_and(|p| p.validate(eng, ctx, actor, version).is_err())
            || rej_payload.as_mut().is_some_and(|p| p.validate(eng, ctx, actor, version).is_err());
        let (outcome, payload) = if invalid {
            (PollOutcome::Cancelled, None)
        } else {
            match policy_res {
                PolicyResult::Accept => (PollOutcome::Accepted, acc_payload),
                PolicyResult::Reject => (PollOutcome::Rejected, rej_payload),
                PolicyResult::Inconclusive => (PollOutcome::Inconclusive, None),
            }
        };

        // Tear the poll down BEFORE running the payload. A resolving payload can itself
        // tear this poll down — a prosecution verdict runs TerminateProsecution, which
        // cleans up its voting poll — so if we cleaned up afterwards we'd double-remove it
        // and desync the validate/execute passes. Cleaning up first means such a payload
        // finds the poll already gone (its own cleanup is guarded on existence) and skips.
        Action::PollCleanup(PollCleanup {
            poll_id: self.poll_id,
            outcome,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        if let Some(mut act) = payload {
            act.handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::PollTimeout(PollTimeoutResponse {}))
    }
}
