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
* Emits no commands of its own. The signal is visible through the ready/done flags the phase
* snapshot carries, which UpdateProsecutions broadcasts on the sweep.
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

        // The flags as they stand BEFORE this call, plus what the phase holds that writing it back
        // needs. Carried out of the match so nothing below re-derives the phase.
        //
        // Debate is matched as a nested pattern so a trial in any other subphase falls through to
        // `_` and errors — it is not a signallable phase.
        let prosecutor_signalled;
        let defense_signalled;
        let timeout_job;
        // Some only in a debate, carrying the trial's channel — also the only phase where
        // signalling shortens a clock.
        let debate_channel;

        match &prosecution.phase {
            ProsecutionPhase::Custody {
                prosecutor_ready,
                defense_ready,
                timeout_job_id,
            } => {
                prosecutor_signalled = *prosecutor_ready;
                defense_signalled = *defense_ready;
                timeout_job = *timeout_job_id;
                debate_channel = None;
            }
            ProsecutionPhase::Trial {
                phase:
                    TrialPhase::Debate {
                        prosecutor_done,
                        defense_done,
                    },
                channel_id,
                timeout_job_id,
            } => {
                prosecutor_signalled = *prosecutor_done;
                defense_signalled = *defense_done;
                timeout_job = *timeout_job_id;
                debate_channel = Some(*channel_id);
            }
            _ => return Err(ActionError::IncompatiblePhase),
        }

        if (is_prosecutor && prosecutor_signalled) || (is_defendant && defense_signalled) {
            return Err(ActionError::AlreadySignalled);
        }

        // The OTHER side had already signalled, so this call completes the pair.
        let resolve =
            (is_prosecutor && defense_signalled) || (is_defendant && prosecutor_signalled);

        // ...and this call's own signal, folded in so the write below records it in either phase.
        let prosecutor_signalled = prosecutor_signalled || is_prosecutor;
        let defense_signalled = defense_signalled || is_defendant;

        if mutate {
            // One side finishing cuts the clock for the other, but only ever downwards — replacing
            // a deadline that is already inside the shortened window would push it back out.
            //
            // Skipped when this call resolves the debate, since the prosecution advances below and
            // a fresh timer would be scheduled only to be discarded.
            let shortened = eng.config.defaults.debate_shortened_timeout;
            let rescheduled = if debate_channel.is_some() && !resolve {
                let curr_job = eng
                    .jobs
                    .view(timeout_job)
                    .expect("expected valid job id to be held within trial phase");
                let remaining = curr_job.request.timestamp - eng.time;

                (remaining > shortened).then(|| {
                    eng.jobs.cancel_id(timeout_job);
                    eng.jobs.push(ActionRequest {
                        actor: ActionActor::System,
                        timestamp: eng.time + shortened,
                        payload: Action::AdvanceProsecution(AdvanceProsecution {
                            prosecution_id: self.prosecution_id,
                        }),
                    })
                })
            } else {
                None
            };

            let timeout_job_id = rescheduled.unwrap_or(timeout_job);
            let prosecution = get_prosecution_mut(eng, self.prosecution_id)
                .expect("prosecution should have already been validated");

            prosecution.phase = match debate_channel {
                Some(channel_id) => ProsecutionPhase::Trial {
                    phase: TrialPhase::Debate {
                        prosecutor_done: prosecutor_signalled,
                        defense_done: defense_signalled,
                    },
                    timeout_job_id,
                    channel_id,
                },
                None => ProsecutionPhase::Custody {
                    prosecutor_ready: prosecutor_signalled,
                    defense_ready: defense_signalled,
                    timeout_job_id,
                },
            };
        }

        if resolve {
            Action::AdvanceProsecution(AdvanceProsecution {
                prosecution_id: self.prosecution_id,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::SignalReady(SignalReadyResponse {}))
    }
}
