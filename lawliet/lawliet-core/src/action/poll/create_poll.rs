/*
* SYSTEM ACTION
* Create a new poll
* (Box is fine because something like this should be as generic as possible for
* developer convenience. This action is rarely used anyway so pointer chasing isn't really a problem.)
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, PollTimeout,
    },
    common::PollKey,
    engine::Engine,
    helpers::get_poll,
    poll::Poll,
    timer::Timer,
};

pub use crate::action::{CreatePoll, CreatePollReponse};

impl ActionInterface for CreatePoll {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        // A ballot with nothing on it can only ever time out inconclusively.
        if self.options.is_empty() {
            return Err(ActionError::PollHasNoOptions);
        }

        let id = if mutate {
            eng.world.add_poll(Poll::new(self))
        } else {
            PollKey::default()
        };

        // The timer fires at the poll, so it cannot exist until the poll has a key — and neither
        // exists off the mutate path. A poll with no duration simply has no timer: it runs until
        // its update policy resolves it or something tears it down.
        if let Some(duration) = self.duration
            && mutate
        {
            let now = eng.time;
            // The clock is gated on the same audience the ballot is: whoever cannot reach the
            // poll is not watching its deadline approach either.
            let gate = get_poll(eng, id).expect("just created").viewport(eng);
            let Engine { world, jobs, .. } = eng;
            let timer = Timer::start(
                jobs,
                now,
                duration,
                gate,
                Action::PollTimeout(PollTimeout { poll_id: id }),
            );
            let timer_id = world.add_timer(timer);
            world.get_poll_mut(id).expect("just created").timer = Some(timer_id);
        }

        super::broadcast_poll(eng, ctx, id, mutate);

        Ok(ActionResponse::CreatePoll(CreatePollReponse { id }))
    }
}
