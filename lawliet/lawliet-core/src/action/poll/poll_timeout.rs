/*
* SYSTEM ACTION
* Handle a poll timeout
* (try to resolve the poll, if it accepts, execute, else clean it up)
*/

use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionExt, ActionInterface, ActionResponse,
        ActionResult, PollCleanup,
    },
    common::Version,
    engine::Engine,
    helpers::get_poll,
    poll::{PolicyResult, PollOutcome},
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
        let mut payloads: SmallVec<[Option<Action>; 4]> =
            poll.options.iter().map(|o| o.payload.clone()).collect();
        let policy_res = poll.timeout_policy(eng);

        // Decide the outcome and which payload (if any) to run — this is also what the frontend
        // is told via ClosePoll. Any option that no longer validates cancels the whole poll
        // instead of resolving it, on the same terms as UpdatePolls.
        let invalid = payloads.iter_mut().any(|payload| {
            payload
                .as_mut()
                .is_some_and(|act| act.validate(eng, ctx, actor, version).is_err())
        });
        let (outcome, payload) = if invalid {
            (PollOutcome::Cancelled, None)
        } else {
            match policy_res {
                PolicyResult::Resolved(option) => (
                    PollOutcome::Resolved(option),
                    payloads.get_mut(option as usize).and_then(Option::take),
                ),
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
