/*
* SYSTEM ACTION
* Check all polls to see if they can be resolved. If they can, resolve them.
*
* A poll has no audience of its own — it rides its parent's viewport (see Poll::viewport), so
* nothing here has to keep one in step. What is left is the two ways a poll can end without
* anybody voting it to a conclusion: its parent going away, and one of its options becoming
* something the engine would refuse to carry out.
*/

use smallvec::{SmallVec, smallvec};

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionExt, ActionInterface, ActionResponse,
        ActionResult, PollCleanup,
    },
    common::PollKey,
    helpers::get_poll,
    poll::{PolicyResult, PollOutcome},
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

        // What each poll ends as, and the payload that ends it. Collected first because
        // resolving one poll can change the answer for another.
        let mut resolutions: SmallVec<[(PollKey, PollOutcome, Option<Action>); 4]> = smallvec![];
        let ids: SmallVec<[PollKey; 8]> = eng.world.polls.keys().collect();
        for id in ids {
            let poll = get_poll(eng, id).unwrap();

            // No parent, no poll. The channel or org this was put to has been torn down, taking
            // the audience and the record with it.
            if poll.viewport(eng).is_none() {
                resolutions.push((id, PollOutcome::Cancelled, None));
                continue;
            }

            let mut payloads: SmallVec<[Option<Action>; 4]> =
                poll.options.iter().map(|o| o.payload.clone()).collect();

            // One option going invalid takes the whole poll with it, rather than the poll losing
            // an option: the ballot was drawn up as a set of choices, and a set that can no
            // longer be offered whole is not the question anybody was asked. Dropping the option
            // instead would also move every index after it out from under the votes already
            // cast.
            let invalid = payloads.iter_mut().any(|payload| {
                payload.as_mut().is_some_and(|act| {
                    act.validate(eng, ctx, &ActionActor::System, version)
                        .is_err()
                })
            });
            if invalid {
                resolutions.push((id, PollOutcome::Cancelled, None));
                continue;
            }

            let poll = get_poll(eng, id).unwrap();
            if let PolicyResult::Resolved(option) = poll.update_policy(eng) {
                let payload = payloads.get_mut(option as usize).and_then(Option::take);
                resolutions.push((id, PollOutcome::Resolved(option), payload));
            }
        }

        // The payloads are guaranteed to succeed by this point. If they don't, something's wrong.
        // All removals go through PollCleanup, which emits ClosePoll with the outcome and tears
        // the poll down.
        //
        // Tear down BEFORE running the payload, on the same terms as PollTimeout: a resolving
        // payload can itself tear this poll down — a prosecution verdict runs TerminateProsecution,
        // which cleans up its own voting poll — and cleaning up afterwards would then find the poll
        // already gone and fail the whole sweep. This way such a payload finds it gone, and its own
        // cleanup is guarded on existence and skips.
        for (id, outcome, payload) in resolutions {
            Action::PollCleanup(PollCleanup {
                poll_id: id,
                outcome,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

            if let Some(mut act) = payload {
                act.handle(eng, ctx, &ActionActor::System, version, mutate)?;
            }
        }

        Ok(ActionResponse::UpdatePolls(UpdatePollsResponse {}))
    }
}
