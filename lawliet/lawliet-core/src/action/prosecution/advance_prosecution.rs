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

use crate::{
    ChannelKey, Time,
    action::{
        ActionInterface, ActionResult, Action, ActionActor, ActionRequest, ActionResponse, CreateChannel, CreatePoll, ProsecutionVoteRes,
    },
    common::{JobID, ProsecutionKey},
    engine::Engine,
    helpers::{get_prosecution, get_prosecution_mut},
    poll::{PollPolicy, PollVisibility, VoterPolicy},
    prosecution::{ProsecutionPhase, TrialPhase, TrialSubphase},
};

fn schedule_advance(eng: &mut Engine, prosecution_id: ProsecutionKey, delay: Time) -> JobID {
    eng.jobs.push(ActionRequest {
        actor: ActionActor::System,
        timestamp: eng.time + delay,
        payload: Action::AdvanceProsecution(AdvanceProsecution { prosecution_id }),
    })
}

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
            ProsecutionPhase::Custody {
                prosecutor_ready,
                defense_ready,
                timeout_job_id,
            } => {
                let job_id = *timeout_job_id;

                let channel_response = Action::CreateChannel(CreateChannel { loggable: true })
                    .handle(eng, ctx, actor, version, mutate)?;
                let ActionResponse::CreateChannel(data) = channel_response else {
                    unreachable!();
                };
                let channel_id = data.id;

                // TODO:
                // evaluate trial channel view perms for every player (new action)

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
                    TrialPhase::Debate {
                        prosecutor_done,
                        defense_done,
                    } => {
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
                            prosecution.phase = ProsecutionPhase::Voting { poll_id: id };
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
