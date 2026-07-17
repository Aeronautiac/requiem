/*
* SYSTEM ACTION
* Create a new poll
* (Box is fine because something like this should be as generic as possible for
* developer convenience. This action is rarely used anyway so pointer chasing isn't really a problem.)
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        PollTimeout, ScheduleJob,
    },
    common::PollKey,
    poll::Poll,
};

pub use crate::action::{CreatePoll, CreatePollReponse};

impl ActionInterface for CreatePoll {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let id = if mutate {
            eng.world.add_poll(Poll::new(
                *(self.accept_payload.clone()),
                *(self.reject_payload.clone()),
                self.visibility,
                self.subject.clone(),
                self.update_policy,
                self.timeout_policy,
                self.voter_policy,
                self.opener,
            ))
        } else {
            PollKey::default()
        };

        // poll only exists in the mutate path
        if let Some(duration) = self.duration
            && mutate
        {
            Action::ScheduleJob(ScheduleJob {
                timestamp: eng.time + duration,
                payload: Box::new(Action::PollTimeout(PollTimeout { poll_id: id })),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        super::broadcast_poll(eng, ctx, id, mutate);

        Ok(ActionResponse::CreatePoll(CreatePollReponse { id }))
    }
}
