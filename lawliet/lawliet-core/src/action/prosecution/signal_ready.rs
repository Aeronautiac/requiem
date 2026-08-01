/*
* PLAYER ACTION
* Signal readiness or completion depending on the current prosecution phase.
*
* Custody phase:
*   Sets the caller's ready flag (prosecutor_ready or defense_ready).
*
* Trial Presentation subphase:
*   The floor-holder ends the slot themselves instead of leaving it to the clock — the slot ends
*   the same either way. Single-sided, so there is no flag to pair: this advances the subphase at
*   once (AdvanceProsecution as System — a subphase move, never a held boundary, refused when
*   frozen). The floor-holder is the prosecutor during Prosecutor(Presentation); during
*   Defense(Presentation) it is the defendant OR their lawyer, the same voice grace_ended_by trusts
*   to open the slot. Any other participant gets NotHoldingFloor. Grace is not handled here — a
*   first message ends it.
*
* Trial Debate subphase:
*   Sets the caller's done flag (prosecutor_done or defense_done).
*   One flag set → countdown shortened.
*
* Setting the flags is all this does. Both being set is what leaves the phase, and that is read by
* UpdateProsecutions on the trailing sweep rather than acted on here: a prosecution that is frozen,
* or non-autonomous and already waiting on a host, cannot move at the moment the second flag lands,
* and the sweep is where it moves once that clears. One trigger, and it is the one that re-runs.
*
* On a non-autonomous prosecution the sweep's advance is held instead, which is what pending_advance
* below then rejects further signals against.
*
* Fails if the prosecution is not in one of the above phases/subphases, or if the caller
* is not a participant in this prosecution.
*
* Emits no commands of its own. The signal is visible through the ready/done flags the phase
* snapshot carries, which UpdateProsecutions broadcasts on the sweep.
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, AdvanceProsecution,
    },
    common::Version,
    engine::Engine,
    helpers::{get_prosecution, get_prosecution_mut, player_id},
    prosecution::{ProsecutionPhase, TrialPhase, TrialSubphase},
};

use super::reschedule_advance;

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
        let is_lawyer = prosecution
            .defense
            .lawyer
            .as_ref()
            .is_some_and(|lawyer| lawyer.actor_id == caller);

        // A Presentation subphase is settled here rather than through the flag logic below: it is
        // single-sided, so nothing pairs and the subphase advances at once. Some(true) means the
        // caller holds the floor, Some(false) a participant who does not; None means this is not a
        // presentation and the paired phases handle it.
        let presentation_holder = match &prosecution.phase {
            ProsecutionPhase::Trial {
                phase: TrialPhase::Prosecutor(TrialSubphase::Presentation),
                ..
            } => Some(is_prosecutor),
            ProsecutionPhase::Trial {
                phase: TrialPhase::Defense(TrialSubphase::Presentation),
                ..
            } => Some(is_defendant || is_lawyer),
            _ => None,
        };

        if let Some(holds_floor) = presentation_holder {
            // A bystander is not in the prosecution at all; keep that distinct from a participant
            // who simply is not the one holding the floor right now.
            if !is_prosecutor && !is_defendant && !is_lawyer {
                return Err(ActionError::NotInProsecution);
            }
            if !holds_floor {
                return Err(ActionError::NotHoldingFloor);
            }
            // AdvanceProsecution enforces the freeze; this only points it at the slot being ended.
            Action::AdvanceProsecution(AdvanceProsecution {
                prosecution_id: self.prosecution_id,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

            return Ok(ActionResponse::SignalReady(SignalReadyResponse {}));
        }

        if !is_prosecutor && !is_defendant {
            return Err(ActionError::NotInProsecution);
        }

        // The phase is already over and waiting on a host; there is nothing left to signal, and in
        // a debate the timer this would try to shorten has already fired.
        if prosecution.pending_advance {
            return Err(ActionError::IncompatiblePhase);
        }

        // The flags as they stand BEFORE this call, plus what the phase holds that writing it back
        // needs. Carried out of the match so nothing below re-derives the phase.
        //
        // Debate is matched as a nested pattern so a trial in any other subphase falls through to
        // `_` and errors — it is not a signallable phase.
        let prosecutor_signalled;
        let defense_signalled;
        let timer;
        // Some only in a debate, carrying the trial's channel — also the only phase where
        // signalling shortens a clock.
        let debate_channel;

        match &prosecution.phase {
            ProsecutionPhase::Custody {
                prosecutor_ready,
                defense_ready,
                timer: phase_timer,
            } => {
                prosecutor_signalled = *prosecutor_ready;
                defense_signalled = *defense_ready;
                timer = *phase_timer;
                debate_channel = None;
            }
            ProsecutionPhase::Trial {
                phase:
                    TrialPhase::Debate {
                        prosecutor_done,
                        defense_done,
                    },
                channel_id,
                timer: phase_timer,
            } => {
                prosecutor_signalled = *prosecutor_done;
                defense_signalled = *defense_done;
                timer = *phase_timer;
                debate_channel = Some(*channel_id);
            }
            _ => return Err(ActionError::IncompatiblePhase),
        }

        if (is_prosecutor && prosecutor_signalled) || (is_defendant && defense_signalled) {
            return Err(ActionError::AlreadySignalled);
        }

        // The OTHER side had already signalled, so this call completes the pair. Only used to skip
        // the shortening below — the advance itself is the sweep's.
        let resolve =
            (is_prosecutor && defense_signalled) || (is_defendant && prosecutor_signalled);

        // ...and this call's own signal, folded in so the write below records it in either phase.
        let prosecutor_signalled = prosecutor_signalled || is_prosecutor;
        let defense_signalled = defense_signalled || is_defendant;

        if mutate {
            // One side finishing cuts the clock for the other, but only ever downwards — replacing
            // a deadline that is already inside the shortened window would push it back out. What
            // is left is asked of the timer rather than of the job, so a debate that spent time
            // frozen is compared on the time it actually had.
            //
            // Skipped when this call resolves the debate, since the trailing sweep advances out of
            // it and a fresh countdown would be started only to be discarded.
            let shortened = eng.config.defaults.debate_shortened_timeout;
            let rescheduled = if debate_channel.is_some() && !resolve {
                let remaining = eng
                    .world
                    .timers
                    .get(timer)
                    .expect("expected a valid timer to be held within the trial phase")
                    .remaining(eng.time);

                (remaining > shortened)
                    .then(|| reschedule_advance(eng, self.prosecution_id, timer, shortened))
            } else {
                None
            };

            let timer = rescheduled.unwrap_or(timer);
            let prosecution = get_prosecution_mut(eng, self.prosecution_id)
                .expect("prosecution should have already been validated");

            prosecution.phase = match debate_channel {
                Some(channel_id) => ProsecutionPhase::Trial {
                    phase: TrialPhase::Debate {
                        prosecutor_done: prosecutor_signalled,
                        defense_done: defense_signalled,
                    },
                    timer,
                    channel_id,
                },
                None => ProsecutionPhase::Custody {
                    prosecutor_ready: prosecutor_signalled,
                    defense_ready: defense_signalled,
                    timer,
                },
            };
        }

        Ok(ActionResponse::SignalReady(SignalReadyResponse {}))
    }
}
