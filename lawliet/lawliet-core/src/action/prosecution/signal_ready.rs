/*
* PLAYER ACTION
* Signal readiness or completion depending on the current prosecution phase.
*
* Custody phase:
*   Sets the caller's ready flag (prosecutor_ready or defense_ready).
*   If both flags are now set, calls AdvanceProsecution (host approval still required
*   if non-autonomous).
*
* Trial Debate subphase:
*   Sets the caller's done flag (prosecutor_done or defense_done).
*   One flag set → timer shortened (reschedule timeout job to a shorter duration).
*   Both flags set → calls AdvanceProsecution immediately (host approval still required
*   if non-autonomous).
*
* Fails if the prosecution is not in one of the above phases/subphases, or if the caller
* is not a participant in this prosecution.
*
* TODO: commands
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionRequest,
        ActionResponse, ActionResult, AdvanceProsecution,
    },
    common::Version,
    engine::Engine,
    helpers::{get_prosecution, get_prosecution_mut, player_id},
    prosecution::{ProsecutionPhase, TrialPhase},
};

pub use crate::action::{SignalReady, SignalReadyResponse};

impl ActionInterface for SignalReady {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.player_only()?;
        let caller = player_id(actor).expect("already validated as player");

        let prosecution = get_prosecution(eng, self.prosecution_id)?;
        let is_prosecutor = prosecution.prosecution.prosecutor == caller;
        let is_defendant = prosecution.defense.defendant == caller;

        if !is_prosecutor && !is_defendant {
            return Err(ActionError::NotInProsecution);
        }

        let mut prosecutor_signalled = false;
        let mut defense_signalled = false;
        let mut advance_debate = false;
        match &prosecution.phase {
            ProsecutionPhase::Custody {
                prosecutor_ready,
                defense_ready,
                timeout_job_id: _,
            } => {
                defense_signalled = *defense_ready;
                prosecutor_signalled = *prosecutor_ready;
            }
            ProsecutionPhase::Trial {
                phase,
                channel_id: _,
                timeout_job_id: _,
            } => {
                if let TrialPhase::Debate {
                    prosecutor_done,
                    defense_done,
                } = phase
                {
                    defense_signalled = *defense_done;
                    prosecutor_signalled = *prosecutor_done;
                    advance_debate = true;
                }
            }
            _ => return Err(ActionError::IncompatiblePhase),
        }
        if (is_prosecutor && prosecutor_signalled) || (is_defendant && defense_signalled) {
            return Err(ActionError::AlreadySignalled);
        }
        let resolve =
            (is_prosecutor && defense_signalled) || (is_defendant && prosecutor_signalled);

        let shortened_debate_time = eng.config.defaults.debate_shortened_timeout;
        if mutate && advance_debate {
            let ProsecutionPhase::Trial {
                timeout_job_id,
                phase: _,
                channel_id,
            } = &prosecution.phase
            else {
                unimplemented!()
            };
            let channel_id = *channel_id;

            // only replace the current timeout if the remaining time is longer than the shortened time
            let curr_job = eng
                .jobs
                .view(*timeout_job_id)
                .expect("expected valid job id to be held within trial phase");
            let remaining_time = curr_job.request.timestamp - eng.time;
            let cancel_job = remaining_time < shortened_debate_time;

            if cancel_job {
                eng.jobs.cancel_id(*timeout_job_id);

                let new_job_id = eng.jobs.push(ActionRequest {
                    actor: ActionActor::System,
                    timestamp: eng.time + shortened_debate_time,
                    payload: Action::AdvanceProsecution(AdvanceProsecution {
                        prosecution_id: self.prosecution_id,
                    }),
                });

                let prosecution = get_prosecution_mut(eng, self.prosecution_id)
                    .expect("prosecution should have already been validated");
                prosecution.phase = ProsecutionPhase::Trial {
                    phase: TrialPhase::Debate {
                        defense_done: defense_signalled,
                        prosecutor_done: prosecutor_signalled,
                    },
                    timeout_job_id: new_job_id,
                    channel_id,
                }
            }
        }

        if resolve {
            Action::AdvanceProsecution(AdvanceProsecution {
                prosecution_id: self.prosecution_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::SignalReady(SignalReadyResponse {}))
    }
}
