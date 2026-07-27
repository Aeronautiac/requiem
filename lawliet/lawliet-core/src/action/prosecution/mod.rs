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
    common::ProsecutionKey,
    engine::Engine,
    helpers::{cmd_world_event, get_prosecution},
};

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

// Tell everyone a prosecution has ended. Addressed the same way as the snapshot, so for an
// absent player it lands after any updates they have yet to receive.
pub(crate) fn broadcast_prosecution_close(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    prosecution_id: ProsecutionKey,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    cmd_world_event(eng, ctx, Command::CloseProsecution { prosecution_id });
}

#[cfg(test)]
mod prosecution_tests {
    use lawliet_types::{
        command::CommandRecipient,
        prosecution::{ProsecutionPhaseView, TrialPhaseView, TrialSubphaseView},
    };

    use crate::{
        action::ActionError,
        channel::ChannelPermission,
        config::role::Role,
        engine::Engine,
        helpers::get_prosecution,
        prosecution::{ProsecutionPhase, TrialPhase},
        test_helpers::{
            add_player, advance_prosecution, init_engine, quick_kill, select_lawyer, signal_ready,
            start_prosecution, terminate_prosecution, update_prosecution_channels,
        },
    };
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
            Command::MapLawyerChannel { channel_id, .. } if *channel_id == channel
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
}
