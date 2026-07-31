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
* - a host, at any moment, from any phase (admin)
*
* On a non-autonomous prosecution the system triggers do not advance the two major boundaries
* (Custody -> Trial, Debate -> Voting); they set pending_advance and return, leaving the
* prosecution where it is until an admin call arrives. An admin call is never held.
*
* A frozen prosecution refuses every system trigger, in every phase, and records nothing. See
* Prosecution::frozen.
*
* All players with presence are added to the trial channel with view permissions (granted via
* deferred commands). Send permissions are restricted to the active side per subphase and are
* set directly — the trial terminates on NoPresence, so there is no case where a player loses
* and then regains send permissions mid-trial.
*
* Transitions:
*   Custody → Trial:
*     replace the custody countdown with the prosecutor's grace countdown
*     create trial channel (loggable)
*     grant view to all present players (just need to evaluate trial channel perms on state change,
*     same as lounges)
*     grant send to prosecutor side only
*     phase = Trial { Prosecutor(Grace), timer }
*
*   Trial Prosecutor(Grace) → Prosecutor(Presentation):
*     replace the grace countdown with the presentation one
*     (send already restricted to prosecutor side)
*
*   Trial Prosecutor(Presentation) → Defense(Grace):
*     restrict send to defense side
*     defense grace countdown
*
*   Trial Defense(Grace) → Defense(Presentation):
*     replace the grace countdown with the presentation one
*     (send already restricted to defense side)
*
*   Trial Defense(Presentation) → Debate:
*     grant send to both sides
*     debate countdown
*     phase = Trial { Debate { prosecutor_done: false, defense_done: false }, timer }
*
*   Trial Debate → Voting (timer expired or both done):
*     revoke all send permissions in trial channel
*     create poll in trial channel
*     phase = Voting { poll_id }
*
* Emits no commands of its own. The phase snapshot is broadcast by UpdateProsecutions on the
* sweep, and channel perms by the SetMembers that UpdateProsecutionChannels issues.
*/

use lawliet_types::channel::{BlueprintDisplayKind, PermUpdatePolicy, ProfileBlueprint, TrialPolicy};

use crate::{
    ChannelKey, Time,
    action::{
        Action, ActionActor, ActionInterface, ActionResponse, ActionResult, CreateAndGiveProfile,
        CreateChannel, CreatePoll, DestroyChannel, ProsecutionVoteRes,
    },
    actor::ActorDisplay,
    channel::ChannelKind,
    command::Command,
    common::{ProsecutionKey, TimerKey},
    engine::Engine,
    helpers::{cmd_channel, get_prosecution, get_prosecution_mut},
    poll::{PollOption, PollOptionLabel, PollParent, PollPolicy, PollSubject, VoterPolicy},
    prosecution::{ProsecutionPhase, TrialPhase, TrialSubphase},
};

use super::reschedule_advance;

// Record that a major boundary was reached and the prosecution is waiting on a host.
//
// Any outstanding timer is left alone. It fires into this same hold and does nothing, the same way
// a debate timer that fires after both sides signalled done lands on a phase it no longer matches.
fn hold_for_host(eng: &mut Engine, mutate: bool, prosecution_id: ProsecutionKey) {
    if mutate {
        get_prosecution_mut(eng, prosecution_id)
            .expect("prosecution was already validated")
            .pending_advance = true;
    }
}

// Channel perms and the client-facing broadcast are handled centrally by UpdateProsecutions in
// the trailing Update step, so this only advances the phase and reschedules the timer.
fn handle_trial_phase(
    eng: &mut Engine,
    mutate: bool,
    prosecution_id: ProsecutionKey,
    timer: TimerKey,
    channel_id: ChannelKey,
    delay: Time,
    new_phase: TrialPhase,
) {
    if mutate {
        let timer = reschedule_advance(eng, prosecution_id, timer, delay);

        let prosecution =
            get_prosecution_mut(eng, prosecution_id).expect("prosecution was already validated");
        prosecution.phase = ProsecutionPhase::Trial {
            phase: new_phase,
            channel_id,
            timer,
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

        // A frozen prosecution does not move at all — not across a boundary, not between subphases.
        // Nothing is recorded, because nothing needs to be: what would have moved it is either the
        // phase's own countdown, which is stopped rather than lost, or both sides having signalled,
        // which is a standing fact UpdateProsecutions re-reads once the freeze lifts.
        //
        // Deliberately not pending_advance. That says a host owes an answer, and an autonomous
        // prosecution would then be waiting forever on someone who is never going to be asked.
        if prosecution.frozen(eng) && actor.is_system() {
            return Ok(ActionResponse::AdvanceProsecution(
                AdvanceProsecutionResponse {},
            ));
        }

        // A non-autonomous prosecution stops at the two major boundaries. Only automatic triggers
        // are held — a host calling this IS the confirmation, and advances from wherever the
        // prosecution currently sits regardless of whether anything asked it to.
        let held = !prosecution.autonomous && actor.is_system();

        match &prosecution.phase {
            ProsecutionPhase::Custody { timer, .. } => {
                if held {
                    hold_for_host(eng, mutate, self.prosecution_id);
                    return Ok(ActionResponse::AdvanceProsecution(
                        AdvanceProsecutionResponse {},
                    ));
                }

                let timer = *timer;
                let prosecutor_id = prosecution.prosecution.prosecutor;
                let prosecutor_display = prosecution.prosecution.prosecutor_display;
                let defendant_id = prosecution.defense.defendant;
                let defendant_display = prosecution.defense.defendant_display;

                // A trial is public, so everyone gets a seat in it, participants included. One
                // policy for every name in the room: it hands out the floor by display, so a
                // prosecutor named openly speaks through the same ordinary seat as everybody else.
                let channel_response = Action::CreateChannel(CreateChannel {
                    loggable: true,
                    base_profile: Some(ProfileBlueprint {
                        start_visible: true,
                        display_kind: BlueprintDisplayKind::OwnerRaw,
                        perm_policy: PermUpdatePolicy::Trial(TrialPolicy {
                            prosecution_id: self.prosecution_id,
                        }),
                    }),
                })
                .handle(eng, ctx, actor, version, mutate)?;
                let ActionResponse::CreateChannel(data) = channel_response else {
                    unreachable!();
                };
                let channel_id = data.id;

                if mutate {
                    // Before anyone is seated: seating enters them into the viewport, and a Map
                    // emitted afterwards would arrive behind history they already hold. Same
                    // ordering rule as the lawyer channel.
                    cmd_channel(
                        eng,
                        ctx,
                        Command::MapChannel {
                            channel_id,
                            kind: ChannelKind::Trial(self.prosecution_id),
                        },
                        channel_id,
                        false,
                        None,
                    );

                    let timer = reschedule_advance(
                        eng,
                        self.prosecution_id,
                        timer,
                        eng.config.defaults.presentation_grace_timeout,
                    );

                    let prosecution = get_prosecution_mut(eng, self.prosecution_id)
                        .expect("prosecution was already validated");
                    prosecution.phase = ProsecutionPhase::Trial {
                        phase: TrialPhase::Prosecutor(TrialSubphase::Grace),
                        channel_id,
                        timer,
                    };
                    prosecution.pending_advance = false;

                    // A participant announced under their own name needs nothing here: the seat
                    // every player already has IS the one that holds the floor, because the policy
                    // matches on display. Only a name the trial invented — an anonymous
                    // prosecutor's mask — has to be brought into existence and handed over.
                    for (player_id, display) in [
                        (prosecutor_id, prosecutor_display),
                        (defendant_id, defendant_display),
                    ] {
                        if display == ActorDisplay::Raw(player_id) {
                            continue;
                        }
                        Action::CreateAndGiveProfile(CreateAndGiveProfile {
                            channel_id,
                            player_id,
                            display,
                            visible: true,
                            shared: false,
                            transferrable: false,
                            perm_policy: PermUpdatePolicy::Trial(TrialPolicy {
                                prosecution_id: self.prosecution_id,
                            }),
                        })
                        .handle(eng, ctx, &ActionActor::System, version, mutate)?;
                    }
                }
            }
            ProsecutionPhase::Trial {
                phase,
                channel_id,
                timer,
            } => {
                let channel_id = *channel_id;
                let timer = *timer;
                match phase {
                    TrialPhase::Prosecutor(subphase) => match subphase {
                        TrialSubphase::Grace => handle_trial_phase(
                            eng,
                            mutate,
                            self.prosecution_id,
                            timer,
                            channel_id,
                            eng.config.defaults.presentation_timeout,
                            TrialPhase::Prosecutor(TrialSubphase::Presentation),
                        ),
                        TrialSubphase::Presentation => handle_trial_phase(
                            eng,
                            mutate,
                            self.prosecution_id,
                            timer,
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
                            timer,
                            channel_id,
                            eng.config.defaults.presentation_timeout,
                            TrialPhase::Defense(TrialSubphase::Presentation),
                        ),
                        TrialSubphase::Presentation => handle_trial_phase(
                            eng,
                            mutate,
                            self.prosecution_id,
                            timer,
                            channel_id,
                            eng.config.defaults.debate_default_timeout,
                            TrialPhase::Debate {
                                prosecutor_done: false,
                                defense_done: false,
                            },
                        ),
                    },
                    TrialPhase::Debate { .. } => {
                        if held {
                            // The floor closes on the hold rather than on the advance:
                            // UpdateProsecutionChannels reads pending_advance, and the trailing
                            // Update step revokes send for both sides before the host ever answers.
                            hold_for_host(eng, mutate, self.prosecution_id);
                            return Ok(ActionResponse::AdvanceProsecution(
                                AdvanceProsecutionResponse {},
                            ));
                        }

                        // A verdict is not an accept and a reject, so it says what it is. Guilty
                        // is option 0.
                        let response = Action::CreatePoll(CreatePoll {
                            options: vec![
                                PollOption {
                                    label: PollOptionLabel::Generic("Guilty".to_string()),
                                    payload: Some(Action::ProsecutionVoteRes(ProsecutionVoteRes {
                                        prosecution_id: self.prosecution_id,
                                        success: true,
                                    })),
                                },
                                PollOption {
                                    label: PollOptionLabel::Generic("Not guilty".to_string()),
                                    payload: Some(Action::ProsecutionVoteRes(ProsecutionVoteRes {
                                        prosecution_id: self.prosecution_id,
                                        success: false,
                                    })),
                                },
                            ],
                            ignore_amplification: false,
                            voter_policy: VoterPolicy::Present,
                            subject: PollSubject::Generic("Trial verdict".to_string()),
                            update_policy: PollPolicy::AlwaysInconclusive,
                            timeout_policy: PollPolicy::MostVoted,
                            parent: PollParent::World,
                            duration: Some(eng.config.defaults.trial_vote_duration),
                            // system-driven verdict vote — no distinct opener
                            opener: None,
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

                        // The defence's private line runs until the verdict does.
                        let lawyer_channel = get_prosecution(eng, self.prosecution_id)
                            .expect("prosecution was already validated")
                            .defense
                            .lawyer
                            .as_ref()
                            .and_then(|lawyer| lawyer.channel_id);

                        if mutate {
                            if let Some(channel_id) = lawyer_channel {
                                Action::DestroyChannel(DestroyChannel { channel_id }).handle(
                                    eng,
                                    ctx,
                                    &ActionActor::System,
                                    version,
                                    mutate,
                                )?;
                            }

                            let prosecution = get_prosecution_mut(eng, self.prosecution_id)
                                .expect("prosecution was already validated");
                            if let Some(lawyer) = &mut prosecution.defense.lawyer {
                                lawyer.channel_id = None;
                            }
                            prosecution.phase = ProsecutionPhase::Voting {
                                poll_id: id,
                                channel_id,
                            };
                            prosecution.pending_advance = false;
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
