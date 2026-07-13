/*
* SYSTEM ACTION
* Check all polls to see if they can be resolved. If they can, resolve them.
*/

use smallvec::{SmallVec, smallvec};

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, ActionExt, PollCleanup,
    },
    common::PollKey,
    helpers::get_poll,
    poll::{PollOutcome, PolicyResult},
};

pub use crate::action::{UpdatePolls, UpdatePollsResponse};

impl ActionInterface for UpdatePolls {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let mut polls_to_cancel: SmallVec<[PollKey; 8]> = smallvec![];
        let mut polls_to_accept: SmallVec<[(PollKey, Option<Action>); 8]> = smallvec![];
        let mut polls_to_reject: SmallVec<[(PollKey, Option<Action>); 8]> = smallvec![];
        let ids: Vec<PollKey> = eng.world.polls.keys().collect();
        for id in ids {
            let poll = get_poll(eng, id).unwrap();
            let mut acc_payload = poll.accept_payload.clone();
            let mut rej_payload = poll.reject_payload.clone();

            if acc_payload.is_some()
                && acc_payload
                    .as_mut()
                    .unwrap()
                    .validate(eng, ctx, &ActionActor::System, version)
                    .is_err()
                || rej_payload.is_some()
                    && rej_payload
                        .as_mut()
                        .unwrap()
                        .validate(eng, ctx, &ActionActor::System, version)
                        .is_err()
            {
                polls_to_cancel.push(id);
            } else {
                let poll = get_poll(eng, id).unwrap();
                let policy_res = poll.update_policy(eng);
                match policy_res {
                    PolicyResult::Accept => polls_to_accept.push((id, acc_payload)),
                    PolicyResult::Reject => polls_to_reject.push((id, rej_payload)),
                    _ => {}
                };
            }
        }

        // All removals go through PollCleanup, which emits ClosePoll with the outcome and
        // tears the poll down.
        for id in polls_to_cancel {
            Action::PollCleanup(PollCleanup {
                poll_id: id,
                outcome: PollOutcome::Cancelled,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        for (id, action) in polls_to_reject {
            if let Some(mut act) = action {
                act.handle(eng, ctx, &ActionActor::System, version, mutate)?;
            }
            Action::PollCleanup(PollCleanup {
                poll_id: id,
                outcome: PollOutcome::Rejected,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        // the actions are guaranteed to succeed by this point. if they dont, something's wrong.
        for (id, action) in polls_to_accept {
            if let Some(mut act) = action {
                act.handle(eng, ctx, &ActionActor::System, version, mutate)?;
            }
            Action::PollCleanup(PollCleanup {
                poll_id: id,
                outcome: PollOutcome::Accepted,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::UpdatePolls(UpdatePollsResponse {}))
    }
}
