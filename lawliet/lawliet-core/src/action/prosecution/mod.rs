pub mod advance_prosecution;
pub mod cull_prosecutions;
pub mod prosecution_vote_res;
pub mod select_lawyer;
pub mod set_custody;
pub mod signal_ready;
pub mod start_prosecution;
pub mod terminate_prosecution;
pub mod update_prosecution_channels;
pub mod update_prosecutions;

use lawliet_types::command::Command;

use crate::{
    action::ActionContext,
    actor::ActorDisplay,
    common::{ActorKey, ChannelKey, ProsecutionKey},
    engine::Engine,
    helpers::{cmd_world_event, get_prosecution},
    prosecution::{ProsecutionPhase, TrialPhase, TrialSubphase},
};

// The prosecution whose trial this message belongs to, if the sender is the side holding the floor
// and has not started their slot yet. A first message is what ends a grace subphase — the slot
// begins when its owner does, not when the clock says so.
//
// Scans the prosecutions rather than indexing channel -> prosecution: there are only ever a handful
// live at once, and an index would be one more thing to keep in step with termination.
pub(crate) fn grace_ended_by(
    eng: &Engine,
    channel_id: ChannelKey,
    sender: ActorKey,
) -> Option<ProsecutionKey> {
    eng.world
        .prosecutions
        .iter()
        .find(|(_, prosecution)| {
            let ProsecutionPhase::Trial {
                phase,
                channel_id: trial_channel,
                ..
            } = &prosecution.phase
            else {
                return false;
            };
            if *trial_channel != channel_id {
                return false;
            }
            match phase {
                TrialPhase::Prosecutor(TrialSubphase::Grace) => {
                    prosecution.prosecution.prosecutor == sender
                }
                // Counsel speaking opens the defence's slot as surely as the defendant does.
                TrialPhase::Defense(TrialSubphase::Grace) => {
                    prosecution.defense.defendant == sender
                        || prosecution
                            .defense
                            .lawyer
                            .as_ref()
                            .is_some_and(|lawyer| lawyer.actor_id == sender)
                }
                _ => false,
            }
        })
        .map(|(key, _)| key)
}

// Broadcast a prosecution's client-facing snapshot to everyone present, plus the System mirror.
//
// The ordered timeline is what matters here, and the presence viewport preserves it for free: a
// player who loses presence exits and stops receiving updates, and re-entry replays every one
// they missed in order. That is what the old deferred queue and the "frozen view" notice were
// both approximating — the queue held the updates, the notice told the client its state was
// stale. Neither is needed now: absence is stated by the exit, and the client already knows
// nothing more will arrive until it re-enters.
pub(crate) fn broadcast_prosecution(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    prosecution_id: ProsecutionKey,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    let Ok(prosecution) = get_prosecution(eng, prosecution_id) else {
        return;
    };
    let (phase, trial_channel) = prosecution.phase_view();
    let prosecutor_display = prosecution.prosecution.prosecutor_display;
    let defendant_display = prosecution.defense.defendant_display;
    // Raw: a defence counsel is a public role, and the anonymity mechanics that give the other two
    // their displays are about the accuser and the accused, not their representation.
    let lawyer_display = prosecution
        .defense
        .lawyer
        .as_ref()
        .map(|lawyer| ActorDisplay::Raw(lawyer.actor_id));

    cmd_world_event(
        eng,
        ctx,
        Command::UpdateProsecution {
            prosecution_id,
            prosecutor_display,
            defendant_display,
            phase,
            trial_channel,
            lawyer_display,
        },
    );
}

// Tell everyone a prosecution has ended, and how. Addressed the same way as the snapshot, so for
// an absent player it lands after any updates they have yet to receive.
//
// The verdict rides this rather than a command of its own because the two always coincide: a
// verdict ends the prosecution, and there is no moment where one is known and the other isn't.
pub(crate) fn broadcast_prosecution_close(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    prosecution_id: ProsecutionKey,
    verdict: Option<bool>,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    cmd_world_event(
        eng,
        ctx,
        Command::CloseProsecution {
            prosecution_id,
            verdict,
        },
    );
}

#[cfg(test)]
mod prosecution_tests {
    use lawliet_types::{
        command::CommandRecipient,
        prosecution::{ProsecutionPhaseView, TrialPhaseView, TrialSubphaseView},
    };

    use crate::{
        action::{Action, ActionActor, ActionError, ActionRequest, AdvanceProsecution},
        actor::ActorDisplay,
        channel::{ChannelKind, ChannelMember, ChannelPermission},
        config::role::Role,
        engine::{Engine, ExecutionResult},
        helpers::{get_channel, get_prosecution},
        prosecution::{ProsecutionPhase, TrialPhase, TrialSubphase},
        test_helpers::{
            add_player, advance_prosecution, create_channel, host_advance_prosecution, init_engine,
            quick_kill, select_lawyer, send_message, set_member, signal_ready, start_prosecution,
            start_prosecution_with, terminate_prosecution, update_prosecution_channels,
        },
    };
    use indexmap::indexset;
    use lawliet_types::command::Command;

    // Prosecutor, defendant, a bystander, and a prosecution of the second by the first.
    fn trial(
        eng: &mut Engine,
    ) -> (
        crate::ActorKey,
        crate::ActorKey,
        crate::ActorKey,
        crate::ProsecutionKey,
    ) {
        init_engine(eng);
        let prosecutor = add_player(eng, 0, Role::Civilian, "prosecutor");
        let defendant = add_player(eng, 0, Role::Civilian, "defendant");
        let bystander = add_player(eng, 0, Role::Civilian, "bystander");
        let id = start_prosecution(eng, 0, prosecutor, defendant);
        (prosecutor, defendant, bystander, id)
    }

    fn custody_flags(eng: &Engine, id: crate::ProsecutionKey) -> (bool, bool) {
        match &get_prosecution(eng, id).unwrap().phase {
            ProsecutionPhase::Custody {
                prosecutor_ready,
                defense_ready,
                ..
            } => (*prosecutor_ready, *defense_ready),
            other => panic!("expected custody, got {other:?}"),
        }
    }

    fn trial_channel(eng: &Engine, id: crate::ProsecutionKey) -> crate::ChannelKey {
        get_prosecution(eng, id)
            .unwrap()
            .phase_view()
            .1
            .expect("trial channel")
    }

    // Same three players, but a prosecution no phase leaves without a host saying so.
    fn non_autonomous_trial(
        eng: &mut Engine,
    ) -> (
        crate::ActorKey,
        crate::ActorKey,
        crate::ActorKey,
        crate::ProsecutionKey,
    ) {
        init_engine(eng);
        let prosecutor = add_player(eng, 0, Role::Civilian, "prosecutor");
        let defendant = add_player(eng, 0, Role::Civilian, "defendant");
        let bystander = add_player(eng, 0, Role::Civilian, "bystander");
        let id = start_prosecution_with(eng, 0, prosecutor, defendant, false);
        (prosecutor, defendant, bystander, id)
    }

    fn speak(
        eng: &mut Engine,
        time: crate::Time,
        speaker: crate::ActorKey,
        id: crate::ProsecutionKey,
    ) -> ExecutionResult {
        let channel = trial_channel(eng, id);
        send_message(
            eng,
            time,
            speaker,
            channel,
            ActorDisplay::Raw(speaker),
            "a word",
        )
    }

    // Resolve the verdict vote directly. Driving the poll itself is the poll protocol's business,
    // not this one's.
    fn vote_res(
        eng: &mut Engine,
        time: crate::Time,
        prosecution_id: crate::ProsecutionKey,
        success: bool,
    ) -> (crate::action::ActionResponse, crate::action::ActionContext) {
        eng.execute(crate::action::ActionRequest {
            actor: crate::action::ActionActor::System,
            timestamp: time,
            payload: crate::action::Action::ProsecutionVoteRes(
                crate::action::ProsecutionVoteRes {
                    prosecution_id,
                    success,
                },
            ),
        })
        .unwrap()
    }

    // ---- signalling ----

    #[test]
    fn custody_signal_is_recorded() {
        let mut eng = Engine::new();
        let (prosecutor, _, _, id) = trial(&mut eng);

        signal_ready(&mut eng, 1, prosecutor, id).unwrap();

        assert_eq!(custody_flags(&eng, id), (true, false));
    }

    // The pair completing is what ends custody, so the first signal has to survive until the
    // second arrives.
    #[test]
    fn both_custody_signals_advance_the_phase() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = trial(&mut eng);

        signal_ready(&mut eng, 1, prosecutor, id).unwrap();
        signal_ready(&mut eng, 2, defendant, id).unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial { .. }
        ));
    }

    #[test]
    fn signalling_twice_is_refused() {
        let mut eng = Engine::new();
        let (prosecutor, _, _, id) = trial(&mut eng);

        signal_ready(&mut eng, 1, prosecutor, id).unwrap();
        let err = signal_ready(&mut eng, 2, prosecutor, id).unwrap_err().0;

        assert!(matches!(err, ActionError::AlreadySignalled));
    }

    #[test]
    fn a_bystander_cannot_signal() {
        let mut eng = Engine::new();
        let (_, _, bystander, id) = trial(&mut eng);

        let err = signal_ready(&mut eng, 1, bystander, id).unwrap_err().0;

        assert!(matches!(err, ActionError::NotInProsecution));
    }

    // Only Custody and the debate subphase accept a signal. A presentation subphase must refuse it
    // rather than silently succeed.
    #[test]
    fn signalling_during_a_presentation_is_refused() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = trial(&mut eng);

        signal_ready(&mut eng, 1, prosecutor, id).unwrap();
        signal_ready(&mut eng, 2, defendant, id).unwrap();
        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial {
                phase: TrialPhase::Prosecutor(_),
                ..
            }
        ));

        let err = signal_ready(&mut eng, 3, prosecutor, id).unwrap_err().0;
        assert!(matches!(err, ActionError::IncompatiblePhase));
    }

    // ---- the wire ----

    #[test]
    fn the_snapshot_carries_custody_flags() {
        let mut eng = Engine::new();
        let (prosecutor, _, _, id) = trial(&mut eng);

        let (_, ctx) = signal_ready(&mut eng, 1, prosecutor, id).unwrap();

        let flags = ctx.commands.iter().find_map(|p| match &p.cmd {
            Command::UpdateProsecution {
                phase:
                    ProsecutionPhaseView::Custody {
                        prosecutor_ready,
                        defense_ready,
                        ..
                    },
                ..
            } => Some((*prosecutor_ready, *defense_ready)),
            _ => None,
        });
        assert_eq!(flags, Some((true, false)));
    }

    // Grace is surfaced rather than collapsed: the side holding the floor has not started yet.
    #[test]
    fn the_snapshot_distinguishes_grace_from_presentation() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = trial(&mut eng);

        signal_ready(&mut eng, 1, prosecutor, id).unwrap();
        let (_, ctx) = signal_ready(&mut eng, 2, defendant, id).unwrap();

        let phase = ctx.commands.iter().rev().find_map(|p| match &p.cmd {
            Command::UpdateProsecution { phase, .. } => Some(*phase),
            _ => None,
        });
        assert_eq!(
            phase,
            Some(ProsecutionPhaseView::Trial(TrialPhaseView::Prosecutor(
                TrialSubphaseView::Grace
            )))
        );
    }

    // Ready flags moving must NOT read as a phase change, or every signal re-announces the trial.
    #[test]
    fn a_signal_is_not_a_phase_change() {
        let custody = |p, d| ProsecutionPhaseView::Custody {
            prosecutor_ready: p,
            defense_ready: d,
            awaiting_host: false,
        };
        assert!(custody(false, false).same_phase(&custody(true, false)));

        let grace =
            ProsecutionPhaseView::Trial(TrialPhaseView::Prosecutor(TrialSubphaseView::Grace));
        let presenting = ProsecutionPhaseView::Trial(TrialPhaseView::Prosecutor(
            TrialSubphaseView::Presentation,
        ));
        assert!(!grace.same_phase(&presenting));
    }

    // ---- lawyers ----

    #[test]
    fn selecting_a_lawyer_makes_a_usable_channel() {
        let mut eng = Engine::new();
        let (_, defendant, bystander, id) = trial(&mut eng);

        let (_, ctx) = select_lawyer(&mut eng, 1, defendant, id, bystander).unwrap();

        let channel = get_prosecution(&eng, id)
            .unwrap()
            .defense
            .lawyer
            .as_ref()
            .expect("lawyer recorded")
            .channel_id
            .expect("channel open while in custody");

        // Introduced to the frontend, or an AddMessage there kills the client.
        assert!(ctx.commands.iter().any(|p| matches!(
            &p.cmd,
            Command::MapChannel { channel_id, kind: ChannelKind::Lawyer(_) } if *channel_id == channel
        )));

        // ...and both parties can actually use it.
        let members = &crate::helpers::get_channel(&eng, channel).unwrap().members;
        assert!(members.contains_key(&defendant));
        assert!(members.contains_key(&bystander));
    }

    #[test]
    fn the_lawyer_reaches_everyone_on_the_snapshot() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, bystander, id) = trial(&mut eng);

        select_lawyer(&mut eng, 1, defendant, id, bystander).unwrap();
        let (_, ctx) = signal_ready(&mut eng, 2, prosecutor, id).unwrap();

        let lawyer = ctx.commands.iter().find_map(|p| match &p.cmd {
            Command::UpdateProsecution { lawyer_display, .. } => Some(*lawyer_display),
            _ => None,
        });
        assert_eq!(
            lawyer,
            Some(Some(crate::actor::ActorDisplay::Raw(bystander)))
        );
    }

    // Contact, not presence: custody leaves contact intact so the accused can still reach counsel,
    // but a dead or incarcerated lawyer must not go on talking to their client.
    #[test]
    fn a_lawyer_who_loses_contact_cannot_send() {
        let mut eng = Engine::new();
        let (_, defendant, bystander, id) = trial(&mut eng);
        select_lawyer(&mut eng, 1, defendant, id, bystander).unwrap();

        let channel = get_prosecution(&eng, id)
            .unwrap()
            .defense
            .lawyer
            .as_ref()
            .unwrap()
            .channel_id
            .unwrap();

        let sendable = |eng: &Engine, who| {
            crate::helpers::get_channel(eng, channel)
                .unwrap()
                .members
                .get(&who)
                .is_some_and(|m| m.perms.contains(ChannelPermission::Send))
        };
        assert!(sendable(&eng, bystander));

        quick_kill(&mut eng, 2, false, true, false, bystander);
        update_prosecution_channels(&mut eng, 3, id);

        assert!(!sendable(&eng, bystander));
        // ...and the client keeps their line, since they are merely bereaved of counsel.
        assert!(sendable(&eng, defendant));
    }

    // The private line runs until the verdict does; who defended the accused outlives it.
    #[test]
    fn voting_closes_the_lawyer_channel_but_keeps_the_lawyer() {
        let mut eng = Engine::new();
        let (_, defendant, bystander, id) = trial(&mut eng);
        select_lawyer(&mut eng, 1, defendant, id, bystander).unwrap();

        let channel = get_prosecution(&eng, id)
            .unwrap()
            .defense
            .lawyer
            .as_ref()
            .unwrap()
            .channel_id
            .unwrap();

        // custody -> prosecutor grace -> presentation -> defense grace -> presentation
        // -> debate -> voting
        for step in 1..=6 {
            advance_prosecution(&mut eng, 1 + step, id);
        }
        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Voting { .. }
        ));

        let lawyer = get_prosecution(&eng, id)
            .unwrap()
            .defense
            .lawyer
            .as_ref()
            .expect("still on record");
        assert_eq!(lawyer.actor_id, bystander);
        assert!(lawyer.channel_id.is_none());
        assert!(crate::helpers::get_channel(&eng, channel).is_err());
    }

    #[test]
    fn the_defendant_cannot_be_their_own_lawyer() {
        let mut eng = Engine::new();
        let (_, defendant, _, id) = trial(&mut eng);

        let err = select_lawyer(&mut eng, 1, defendant, id, defendant)
            .unwrap_err()
            .0;

        assert!(matches!(err, ActionError::CannotBeOwnLawyer));
    }

    #[test]
    fn only_the_defendant_picks_the_lawyer() {
        let mut eng = Engine::new();
        let (prosecutor, _, bystander, id) = trial(&mut eng);

        let err = select_lawyer(&mut eng, 1, prosecutor, id, bystander)
            .unwrap_err()
            .0;

        assert!(matches!(err, ActionError::NotInProsecution));
    }

    #[test]
    fn a_lawyer_cannot_be_replaced() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, bystander, id) = trial(&mut eng);

        select_lawyer(&mut eng, 1, defendant, id, bystander).unwrap();
        let err = select_lawyer(&mut eng, 2, defendant, id, prosecutor)
            .unwrap_err()
            .0;

        assert!(matches!(err, ActionError::LawyerAlreadySelected));
    }

    // ---- lifecycle ----

    #[test]
    fn a_prosecution_is_gone_after_termination() {
        let mut eng = Engine::new();
        let (_, _, _, id) = trial(&mut eng);

        terminate_prosecution(&mut eng, 1, id).unwrap();

        assert!(get_prosecution(&eng, id).is_err());
    }

    #[test]
    fn advancing_out_of_custody_opens_a_trial_channel() {
        let mut eng = Engine::new();
        let (_, _, _, id) = trial(&mut eng);

        advance_prosecution(&mut eng, 1, id);

        let (_, channel) = get_prosecution(&eng, id).unwrap().phase_view();
        assert!(channel.is_some());
    }

    // The trial channel is registered on its OWN viewport. UpdateProsecution names it too, but that
    // rides the presence viewport, so a frontend registering it from there files every message sent
    // in the trial against the wrong viewport.
    #[test]
    fn the_trial_channel_is_mapped_on_its_own_viewport() {
        let mut eng = Engine::new();
        let (_, _, _, id) = trial(&mut eng);

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 1,
                payload: Action::AdvanceProsecution(AdvanceProsecution {
                    prosecution_id: id,
                }),
            })
            .unwrap();

        let channel = trial_channel(&eng, id);
        let membership = get_channel(&eng, channel).unwrap().membership_viewport;

        assert!(ctx.commands.iter().any(|p| matches!(
            (&p.cmd, &p.recipient),
            (
                Command::MapChannel { channel_id, kind: ChannelKind::Trial(_) },
                CommandRecipient::Viewport(v),
            ) if *channel_id == channel && *v == membership
        )));
    }

    // Addressed to the presence viewport rather than broadcast, so an absent player receives it on
    // return instead of never.
    #[test]
    fn the_close_is_addressed_to_the_presence_viewport() {
        let mut eng = Engine::new();
        let (_, _, _, id) = trial(&mut eng);
        let presence = eng.world.presence_viewport;

        let (_, ctx) = terminate_prosecution(&mut eng, 1, id).unwrap();

        assert!(ctx.commands.iter().any(|p| matches!(
            (&p.cmd, &p.recipient),
            (Command::CloseProsecution { .. }, CommandRecipient::Viewport(v)) if *v == presence
        )));
    }

    // ---- grace ----

    // A slot begins when its owner does. Speaking is the whole point of holding the floor, so the
    // first word ends the grace period rather than waiting out a timer nobody is watching.
    #[test]
    fn speaking_ends_the_prosecutor_grace() {
        let mut eng = Engine::new();
        let (prosecutor, _, _, id) = trial(&mut eng);
        advance_prosecution(&mut eng, 1, id);

        speak(&mut eng, 2, prosecutor, id).unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial {
                phase: TrialPhase::Prosecutor(TrialSubphase::Presentation),
                ..
            }
        ));
    }

    // Counsel opens the defence's slot as surely as the defendant does.
    #[test]
    fn counsel_speaking_ends_the_defense_grace() {
        let mut eng = Engine::new();
        let (_, defendant, bystander, id) = trial(&mut eng);
        select_lawyer(&mut eng, 1, defendant, id, bystander).unwrap();

        // custody -> prosecutor grace -> presentation -> defense grace
        for step in 1..=3 {
            advance_prosecution(&mut eng, 1 + step, id);
        }

        speak(&mut eng, 6, bystander, id).unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial {
                phase: TrialPhase::Defense(TrialSubphase::Presentation),
                ..
            }
        ));
    }

    // Speaking during a presentation has nothing left to start.
    #[test]
    fn speaking_during_a_presentation_changes_nothing() {
        let mut eng = Engine::new();
        let (prosecutor, _, _, id) = trial(&mut eng);
        advance_prosecution(&mut eng, 1, id);
        speak(&mut eng, 2, prosecutor, id).unwrap();

        speak(&mut eng, 3, prosecutor, id).unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial {
                phase: TrialPhase::Prosecutor(TrialSubphase::Presentation),
                ..
            }
        ));
    }

    // The trial channel is the only channel that drives a phase. A prosecutor talking elsewhere is
    // just talking.
    #[test]
    fn speaking_in_another_channel_does_not_end_grace() {
        let mut eng = Engine::new();
        let (prosecutor, _, _, id) = trial(&mut eng);
        advance_prosecution(&mut eng, 2, id);

        let elsewhere = create_channel(&mut eng, 3, false);
        set_member(
            &mut eng,
            3,
            prosecutor,
            elsewhere,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(prosecutor)],
            }),
        )
        .unwrap();

        send_message(
            &mut eng,
            4,
            prosecutor,
            elsewhere,
            ActorDisplay::Raw(prosecutor),
            "a word",
        )
        .unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial {
                phase: TrialPhase::Prosecutor(TrialSubphase::Grace),
                ..
            }
        ));
    }

    // ---- host approval ----

    // Both sides ready is the condition, not the decision. A non-autonomous custody stays put.
    #[test]
    fn a_non_autonomous_custody_holds_for_a_host() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = non_autonomous_trial(&mut eng);

        signal_ready(&mut eng, 1, prosecutor, id).unwrap();
        signal_ready(&mut eng, 2, defendant, id).unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Custody { .. }
        ));
        assert!(get_prosecution(&eng, id).unwrap().pending_advance);
    }

    // The wait is shown, so a stalled trial reads as deliberate rather than broken.
    #[test]
    fn the_snapshot_shows_the_wait() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = non_autonomous_trial(&mut eng);

        signal_ready(&mut eng, 1, prosecutor, id).unwrap();
        let (_, ctx) = signal_ready(&mut eng, 2, defendant, id).unwrap();

        let phase = ctx.commands.iter().rev().find_map(|p| match &p.cmd {
            Command::UpdateProsecution { phase, .. } => Some(*phase),
            _ => None,
        });
        assert_eq!(
            phase,
            Some(ProsecutionPhaseView::Custody {
                prosecutor_ready: true,
                defense_ready: true,
                awaiting_host: true,
            })
        );
    }

    #[test]
    fn a_host_releases_a_held_custody() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = non_autonomous_trial(&mut eng);
        signal_ready(&mut eng, 1, prosecutor, id).unwrap();
        signal_ready(&mut eng, 2, defendant, id).unwrap();

        host_advance_prosecution(&mut eng, 3, id).unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial { .. }
        ));
        assert!(!get_prosecution(&eng, id).unwrap().pending_advance);
    }

    // A host does not have to wait for the condition either — the confirmation IS the decision.
    #[test]
    fn a_host_advances_a_custody_nobody_asked_to_end() {
        let mut eng = Engine::new();
        let (_, _, _, id) = non_autonomous_trial(&mut eng);

        host_advance_prosecution(&mut eng, 1, id).unwrap();

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial { .. }
        ));
    }

    // Only the two major boundaries wait. Holding the trial's own subphases would mean approving
    // every time someone starts talking.
    #[test]
    fn subphases_never_wait_for_a_host() {
        let mut eng = Engine::new();
        let (prosecutor, _, _, id) = non_autonomous_trial(&mut eng);
        host_advance_prosecution(&mut eng, 1, id).unwrap();

        speak(&mut eng, 2, prosecutor, id).unwrap();
        advance_prosecution(&mut eng, 3, id);

        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial {
                phase: TrialPhase::Defense(TrialSubphase::Grace),
                ..
            }
        ));
        assert!(!get_prosecution(&eng, id).unwrap().pending_advance);
    }

    // A held debate is over in everything but the confirmation, so the floor closes without it.
    #[test]
    fn a_held_debate_closes_the_floor() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = non_autonomous_trial(&mut eng);
        host_advance_prosecution(&mut eng, 1, id).unwrap();
        // prosecutor grace -> presentation -> defense grace -> presentation -> debate
        for step in 1..=4 {
            advance_prosecution(&mut eng, 1 + step, id);
        }

        signal_ready(&mut eng, 7, prosecutor, id).unwrap();
        signal_ready(&mut eng, 8, defendant, id).unwrap();

        let prosecution = get_prosecution(&eng, id).unwrap();
        assert!(matches!(
            prosecution.phase,
            ProsecutionPhase::Trial {
                phase: TrialPhase::Debate { .. },
                ..
            }
        ));
        assert!(prosecution.pending_advance);

        let channel = get_channel(&eng, trial_channel(&eng, id)).unwrap();
        for side in [prosecutor, defendant] {
            assert!(
                !channel
                    .get_member(side)
                    .unwrap()
                    .perms
                    .contains(ChannelPermission::Send),
                "a held debate must not leave the floor open"
            );
        }
    }

    // Nothing left to signal: the phase is over and only the host is missing. Reached here by the
    // custody timeout, so neither side has signalled yet and the refusal is the hold's doing rather
    // than a repeat signal's.
    #[test]
    fn signalling_into_a_held_phase_is_refused() {
        let mut eng = Engine::new();
        let (prosecutor, defendant, _, id) = non_autonomous_trial(&mut eng);

        advance_prosecution(&mut eng, 1, id);
        assert_eq!(custody_flags(&eng, id), (false, false));
        assert!(get_prosecution(&eng, id).unwrap().pending_advance);

        for side in [prosecutor, defendant] {
            let err = signal_ready(&mut eng, 2, side, id).unwrap_err().0;
            assert!(matches!(err, ActionError::IncompatiblePhase));
        }
    }

    #[test]
    fn an_autonomous_prosecution_is_never_held() {
        let mut eng = Engine::new();
        let (_, _, _, id) = trial(&mut eng);

        advance_prosecution(&mut eng, 1, id);

        assert!(!get_prosecution(&eng, id).unwrap().pending_advance);
        assert!(matches!(
            get_prosecution(&eng, id).unwrap().phase,
            ProsecutionPhase::Trial { .. }
        ));
    }

    // ---- verdicts ----

    fn close_verdict(ctx: &crate::action::ActionContext) -> Option<Option<bool>> {
        ctx.commands.iter().find_map(|p| match &p.cmd {
            Command::CloseProsecution { verdict, .. } => Some(*verdict),
            _ => None,
        })
    }

    #[test]
    fn a_termination_closes_without_a_verdict() {
        let mut eng = Engine::new();
        let (_, _, _, id) = trial(&mut eng);

        let (_, ctx) = terminate_prosecution(&mut eng, 1, id).unwrap();

        assert_eq!(close_verdict(&ctx), Some(None));
    }

    // An acquittal has no other trace — nobody dies — so the close is the only thing that says it
    // happened.
    #[test]
    fn an_acquittal_reaches_the_wire() {
        let mut eng = Engine::new();
        let (_, defendant, _, id) = trial(&mut eng);

        let (_, ctx) = vote_res(&mut eng, 1, id, false);

        assert_eq!(close_verdict(&ctx), Some(Some(false)));
        assert!(
            !crate::helpers::get_actor(&eng, defendant)
                .unwrap()
                .has_state(crate::actor::state::State::Dead)
        );
    }

    #[test]
    fn a_guilty_verdict_executes_the_defendant() {
        let mut eng = Engine::new();
        let (_, defendant, _, id) = trial(&mut eng);

        let (_, ctx) = vote_res(&mut eng, 1, id, true);

        assert_eq!(close_verdict(&ctx), Some(Some(true)));
        assert!(
            crate::helpers::get_actor(&eng, defendant)
                .unwrap()
                .has_state(crate::actor::state::State::Dead)
        );
    }

    // The condemned hears the verdict. Death takes presence, and the close is a world event, so
    // announcing after the execution would tell everyone the outcome except the one it was on.
    #[test]
    fn the_defendant_hears_the_verdict_before_the_execution() {
        let mut eng = Engine::new();
        let (_, defendant, _, id) = trial(&mut eng);

        let (_, ctx) = vote_res(&mut eng, 1, id, true);

        let closed = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::CloseProsecution { .. }))
            .expect("closed");
        let exited = ctx
            .commands
            .iter()
            .position(
                |p| matches!(&p.cmd, Command::ExitViewport { actor, .. } if *actor == defendant),
            )
            .expect("lost presence");

        assert!(
            closed < exited,
            "closed at {closed}, but the defendant had already left at {exited}"
        );
    }
}
