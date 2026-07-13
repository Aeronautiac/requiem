/*
* SYSTEM / ADMIN ACTION
* Drive the prosecution state machine forward by one phase or subphase.
*
* Called by:
* - custody timeout job (system)
* - both ready flags set in Custody (system, after SignalReady)
* - first message sent by the active side during a Grace subphase (system, from SendMessage)
* - presentation/debate timeout job (system)
* - both done flags set in Debate (system, after SignalDone)
* - host manually advancing a non-autonomous prosecution (admin)
*
* All players with presence are added to the trial channel with view permissions (granted via
* deferred commands). Send permissions are restricted to the active side per subphase and are
* set directly — the trial terminates on NoPresence, so there is no case where a player loses
* and then regains send permissions mid-trial.
*
* Transitions:
*   Custody → Trial:
*     cancel custody timeout job
*     create trial channel (loggable)
*     grant view to all present players (just need to evaluate trial channel perms on state change,
*     same as lounges)
*     grant send to prosecutor side only
*     schedule prosecutor grace timeout → timeout_job_id
*     phase = Trial { Prosecutor(Grace), timeout_job_id }
*
*   Trial Prosecutor(Grace) → Prosecutor(Presentation):
*     cancel grace job, schedule presentation timeout
*     (send already restricted to prosecutor side)
*
*   Trial Prosecutor(Presentation) → Defense(Grace):
*     restrict send to defense side
*     schedule defense grace timeout
*
*   Trial Defense(Grace) → Defense(Presentation):
*     cancel grace job, schedule presentation timeout
*     (send already restricted to defense side)
*
*   Trial Defense(Presentation) → Debate:
*     grant send to both sides
*     schedule debate timeout
*     phase = Trial { Debate { prosecutor_done: false, defense_done: false }, timeout_job_id }
*
*   Trial Debate → Voting (timer expired or both done):
*     revoke all send permissions in trial channel
*     create poll in trial channel
*     phase = Voting { poll_id }
*
* TODO: commands
*/

use indexmap::indexset;

use crate::{
    ActorKey, ChannelKey, Time,
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionRequest, ActionResponse, CreateChannel, CreatePoll, ProsecutionVoteRes, SetMember,
    },
    actor::ActorDisplay,
    channel::{ChannelMember, ChannelPermissions},
    common::{JobID, ProsecutionKey, Version},
    engine::Engine,
    helpers::{get_prosecution, get_prosecution_mut},
    poll::{PollPolicy, PollSubject, PollVisibility, VoterPolicy},
    prosecution::{ProsecutionPhase, TrialPhase, TrialSubphase},
};

fn schedule_advance(eng: &mut Engine, prosecution_id: ProsecutionKey, delay: Time) -> JobID {
    eng.jobs.push(ActionRequest {
        actor: ActionActor::System,
        timestamp: eng.time + delay,
        payload: Action::AdvanceProsecution(AdvanceProsecution { prosecution_id }),
    })
}

// Seed a key participant onto the freshly created trial channel with empty perms and a fixed
// display. The trailing UpdateProsecutions step grants the real view/send perms, preserving this
// display rather than falling back to Raw.
fn seed_member(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    version: Version,
    mutate: bool,
    channel_id: ChannelKey,
    player_id: ActorKey,
    display: ActorDisplay,
) -> ActionResult {
    Action::SetMember(SetMember {
        player_id,
        channel_id,
        settings: Some(ChannelMember {
            perms: ChannelPermissions::EMPTY,
            displays: indexset![display],
        }),
    })
    .handle(eng, ctx, &ActionActor::System, version, mutate)
}

// Channel perms and the client-facing broadcast are handled centrally by UpdateProsecutions in
// the trailing Update step, so this only advances the phase and reschedules the timer.
fn handle_trial_phase(
    eng: &mut Engine,
    mutate: bool,
    prosecution_id: ProsecutionKey,
    timeout_job_id: JobID,
    channel_id: ChannelKey,
    delay: Time,
    new_phase: TrialPhase,
) {
    if mutate {
        eng.jobs.cancel_id(timeout_job_id);
        let job_id = schedule_advance(eng, prosecution_id, delay);

        let prosecution =
            get_prosecution_mut(eng, prosecution_id).expect("prosecution was already validated");
        prosecution.phase = ProsecutionPhase::Trial {
            phase: new_phase,
            channel_id,
            timeout_job_id: job_id,
        };
    }
}

pub use crate::action::{AdvanceProsecution, AdvanceProsecutionResponse};

impl ActionInterface for AdvanceProsecution {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let prosecution = get_prosecution(eng, self.prosecution_id)?;
        match &prosecution.phase {
            ProsecutionPhase::Custody { timeout_job_id, .. } => {
                let job_id = *timeout_job_id;
                let prosecutor_id = prosecution.prosecution.prosecutor;
                let prosecutor_display = prosecution.prosecution.prosecutor_display;
                let defendant_id = prosecution.defense.defendant;
                let defendant_display = prosecution.defense.defendant_display;
                let lawyer_id = prosecution.defense.lawyer.as_ref().map(|l| l.actor_id);

                let channel_response = Action::CreateChannel(CreateChannel { loggable: true })
                    .handle(eng, ctx, actor, version, mutate)?;
                let ActionResponse::CreateChannel(data) = channel_response else {
                    unreachable!();
                };
                let channel_id = data.id;

                if mutate {
                    eng.jobs.cancel_id(job_id);
                    let job_id = schedule_advance(
                        eng,
                        self.prosecution_id,
                        eng.config.defaults.presentation_grace_timeout,
                    );

                    let prosecution = get_prosecution_mut(eng, self.prosecution_id)
                        .expect("prosecution was already validated");
                    prosecution.phase = ProsecutionPhase::Trial {
                        phase: TrialPhase::Prosecutor(TrialSubphase::Grace),
                        channel_id,
                        timeout_job_id: job_id,
                    };

                    // Seed the key participants with their displays and empty perms. The trailing
                    // UpdateProsecutions step grants view/send perms across all present players.
                    seed_member(
                        eng, ctx, version, mutate, channel_id, prosecutor_id, prosecutor_display,
                    )?;
                    seed_member(
                        eng, ctx, version, mutate, channel_id, defendant_id, defendant_display,
                    )?;
                    if let Some(lawyer_id) = lawyer_id {
                        seed_member(
                            eng,
                            ctx,
                            version,
                            mutate,
                            channel_id,
                            lawyer_id,
                            ActorDisplay::Raw(lawyer_id),
                        )?;
                    }
                }
            }
            ProsecutionPhase::Trial {
                phase,
                channel_id,
                timeout_job_id,
            } => {
                let channel_id = *channel_id;
                let timeout_job_id = *timeout_job_id;
                match phase {
                    TrialPhase::Prosecutor(subphase) => match subphase {
                        TrialSubphase::Grace => handle_trial_phase(
                            eng,
                            mutate,
                            self.prosecution_id,
                            timeout_job_id,
                            channel_id,
                            eng.config.defaults.presentation_timeout,
                            TrialPhase::Prosecutor(TrialSubphase::Presentation),
                        ),
                        TrialSubphase::Presentation => handle_trial_phase(
                            eng,
                            mutate,
                            self.prosecution_id,
                            timeout_job_id,
                            channel_id,
                            eng.config.defaults.presentation_grace_timeout,
                            TrialPhase::Defense(TrialSubphase::Grace),
                        ),
                    },
                    TrialPhase::Defense(subphase) => match subphase {
                        TrialSubphase::Grace => handle_trial_phase(
                            eng,
                            mutate,
                            self.prosecution_id,
                            timeout_job_id,
                            channel_id,
                            eng.config.defaults.presentation_timeout,
                            TrialPhase::Defense(TrialSubphase::Presentation),
                        ),
                        TrialSubphase::Presentation => handle_trial_phase(
                            eng,
                            mutate,
                            self.prosecution_id,
                            timeout_job_id,
                            channel_id,
                            eng.config.defaults.debate_default_timeout,
                            TrialPhase::Debate {
                                prosecutor_done: false,
                                defense_done: false,
                            },
                        ),
                    },
                    TrialPhase::Debate { .. } => {
                        let response = Action::CreatePoll(CreatePoll {
                            accept_payload: Box::new(Some(Action::ProsecutionVoteRes(
                                ProsecutionVoteRes {
                                    prosecution_id: self.prosecution_id,
                                    success: true,
                                },
                            ))),
                            reject_payload: Box::new(Some(Action::ProsecutionVoteRes(
                                ProsecutionVoteRes {
                                    prosecution_id: self.prosecution_id,
                                    success: false,
                                },
                            ))),
                            voter_policy: VoterPolicy::Present,
                            subject: PollSubject::Generic("Trial verdict".to_string()),
                            update_policy: PollPolicy::AlwaysInconclusive,
                            timeout_policy: PollPolicy::WinningVote,
                            visibility: PollVisibility::AllPresent,
                            duration: Some(eng.config.defaults.trial_vote_duration),
                        })
                        .handle(
                            eng,
                            ctx,
                            &ActionActor::System,
                            version,
                            mutate,
                        )?;
                        let ActionResponse::CreatePoll(create_poll_response) = response else {
                            unreachable!();
                        };
                        let id = create_poll_response.id;

                        if mutate {
                            let prosecution = get_prosecution_mut(eng, self.prosecution_id)
                                .expect("prosecution was already validated");
                            prosecution.phase = ProsecutionPhase::Voting {
                                poll_id: id,
                                channel_id,
                            };
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(ActionResponse::AdvanceProsecution(
            AdvanceProsecutionResponse {},
        ))
    }
}
