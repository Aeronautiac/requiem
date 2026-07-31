/*
* A duration that can be stopped and started again.
*
* The job queue holds absolute timestamps, so a scheduled job cannot be paused — it can only be
* cancelled and pushed again later for what is left. Working out what is left (how much of the
* duration a run served, how much is still owed) is what this type owns, so nothing that needs a
* pausable deadline has to keep its own start time and bank its own remainder.
*
* Timers are objects in the world with keys of their own, not fields on the things they time. What
* stops them is a sweep over every timer in existence, and a sweep should not have to know that a
* poll keeps its countdown in one place and a trial in another — it asks the timer whether it may
* be stopped and stops it. Whoever owns a timer holds its key and is responsible for freeing it,
* the same arrangement viewports have.
*
* A timer carries the action it fires, which is the other half of that: the sweep pauses and
* resumes timers without knowing whether it is stopping a trial or a vote.
*
* Every timer fires as System. A timer is the world acting on its own schedule; the passage of
* time is nobody's action.
*/

use crate::{
    Time,
    action::{Action, ActionActor, ActionRequest},
    common::{JobID, ViewportKey},
    engine::jobs::Jobs,
};

// The current run. Both halves live and die together — cancelling the job is what stops the clock,
// and the start time means nothing once it has.
#[derive(Debug)]
struct TimerRun {
    started_at: Time,
    job: JobID,
}

#[derive(Debug)]
pub struct Timer {
    payload: Action,
    duration: Time,
    // The audience this countdown is for. While that viewport holds nobody the clock stops, and
    // the time it stood still is handed back — a deadline nobody could watch approach is not one
    // they were given. None runs regardless of who is watching.
    //
    // A viewport rather than a reason: every way an audience can be taken away already ends in
    // one emptying, so this asks about the outcome instead of enumerating the causes.
    pub gate: Option<ViewportKey>,
    // Served by runs that have already ended. The current run's share joins it on pause.
    served: Time,
    // None while paused.
    run: Option<TimerRun>,
}

impl Timer {
    // Timers are created running. A timer that has never run has nothing to resume into, so there
    // is no paused constructor: whoever wants one starts it and pauses it.
    pub fn start(
        jobs: &mut Jobs,
        now: Time,
        duration: Time,
        gate: Option<ViewportKey>,
        payload: Action,
    ) -> Self {
        let mut timer = Timer {
            payload,
            duration,
            gate,
            served: 0,
            run: None,
        };
        timer.schedule(jobs, now);
        timer
    }

    pub fn is_paused(&self) -> bool {
        self.run.is_none()
    }

    // What is left of the duration. Saturates at zero: a timer never owes negative time.
    pub fn remaining(&self, now: Time) -> Time {
        let current = self
            .run
            .as_ref()
            .map_or(0, |run| now.saturating_sub(run.started_at));
        self.duration.saturating_sub(self.served + current)
    }

    fn schedule(&mut self, jobs: &mut Jobs, now: Time) {
        let job = jobs.push(ActionRequest {
            actor: ActionActor::System,
            timestamp: now + self.remaining(now),
            payload: self.payload.clone(),
        });
        self.run = Some(TimerRun {
            started_at: now,
            job,
        });
    }

    // Stop the clock and bank what this run served. Idempotent, because the sweep that drives this
    // recomputes rather than tracks: it pauses an already-paused timer on every pass but the first.
    pub fn pause(&mut self, jobs: &mut Jobs, now: Time) {
        let Some(run) = self.run.take() else {
            return;
        };
        self.served += now.saturating_sub(run.started_at);
        jobs.cancel_id(run.job);
    }

    // Start the clock again on whatever is left. Idempotent for the same reason as pause.
    pub fn resume(&mut self, jobs: &mut Jobs, now: Time) {
        if self.run.is_some() {
            return;
        }
        self.schedule(jobs, now);
    }

    // Stop for good. For a timer being freed, which must not fire into the absence of whatever
    // it was counting down for.
    pub fn cancel(&mut self, jobs: &mut Jobs) {
        if let Some(run) = self.run.take() {
            jobs.cancel_id(run.job);
        }
    }
}
