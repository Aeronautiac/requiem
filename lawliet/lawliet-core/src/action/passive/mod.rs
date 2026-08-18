pub mod add_passive;
pub mod create_and_give_passive;
pub mod destroy_passive;
pub mod give_passive;
pub mod take_passive;

#[cfg(test)]
mod contact_log_tests {
    use lawliet_types::{
        command::{Command, CommandRecipient},
        passive::{ContactEvent, ContactLogType},
    };

    use crate::{
        ActorKey,
        action::{Action, ActionActor, ActionContext, ActionRequest},
        actor::ActorDisplay,
        common::{ID, ViewportKey},
        config::role::Role,
        engine::Engine,
        lounge::LoungeVariant,
        passive::PassiveType,
        test_helpers::{add_player, init_engine, quick_kill, quick_passive},
    };

    fn basic_lounge(
        eng: &mut Engine,
        time: crate::Time,
        contactor: ActorKey,
        contacted: ActorKey,
    ) -> ActionContext {
        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateLounge(crate::action::CreateLounge {
                variant: LoungeVariant::Basic {
                    contactor_id: contactor,
                    contacted_id: contacted,
                },
            }),
        }, Engine::version())
        .unwrap()
        .1
    }

    // A world-level contact-log record. It exists from world creation and belongs to no passive;
    // holding the matching passive is only what enters an actor into it.
    fn the_log(eng: &Engine, kind: ContactLogType) -> ViewportKey {
        eng.world.contact_log_viewport(kind)
    }

    fn members(eng: &Engine, viewport: ViewportKey) -> Vec<ActorKey> {
        eng.world
            .get_viewport(viewport)
            .expect("a world log viewport")
            .members()
            .collect()
    }

    // Full sees every contact, so filtering to it gives exactly one row per contact — the parity
    // copy carries identical content and would only double every count.
    fn logged(ctx: &ActionContext) -> Vec<(ID, ActorDisplay, ActorDisplay, ContactEvent)> {
        ctx.commands
            .iter()
            .filter_map(|p| match &p.cmd {
                Command::AddContactLog {
                    kind: ContactLogType::Full,
                    log,
                } => Some((log.contact_id, log.contactor, log.contacted, log.event)),
                _ => None,
            })
            .collect()
    }

    // ---- what lands in the log ----

    #[test]
    fn a_lounge_logs_who_reached_whom() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        add_player(&mut eng, 0, Role::Watari, "watari");
        let a = add_player(&mut eng, 0, Role::Civilian, "a");
        let b = add_player(&mut eng, 0, Role::Civilian, "b");

        let ctx = basic_lounge(&mut eng, 1, a, b);

        let entries = logged(&ctx);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].1..=entries[0].2,
            ActorDisplay::Raw(a)..=ActorDisplay::Raw(b)
        );
        assert_eq!(entries[0].3, ContactEvent::LoungeOpened);
    }

    // An anonymous lounge is logged as the ROLE it presents, not the player behind it. A log that
    // resolved the display would defeat the ability outright.
    #[test]
    fn an_anonymous_lounge_logs_the_role_it_shows() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        add_player(&mut eng, 0, Role::Watari, "watari");
        let a = add_player(&mut eng, 0, Role::Civilian, "a");
        let b = add_player(&mut eng, 0, Role::Civilian, "b");

        let ctx = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 1,
                payload: Action::CreateLounge(crate::action::CreateLounge {
                    variant: LoungeVariant::Anonymous {
                        contactor_id: a,
                        contacted_id: b,
                        role_display: lawliet_types::lounge::AnonymousLoungeRoleDisplay::Static(
                            Role::Kira,
                        ),
                    },
                }),
            }, Engine::version())
            .unwrap()
            .1;

        let entries = logged(&ctx);
        assert_eq!(entries[0].1, ActorDisplay::Role(Role::Kira));
        assert_eq!(entries[0].2, ActorDisplay::Raw(b));
    }

    // ---- the even/odd split ----

    #[test]
    fn even_and_odd_split_on_the_contact_id() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let a = add_player(&mut eng, 0, Role::Civilian, "a");
        let b = add_player(&mut eng, 0, Role::Civilian, "b");

        let first = basic_lounge(&mut eng, 1, a, b);
        let second = basic_lounge(&mut eng, 2, b, a);

        // Which of the three records each contact was written to.
        let kinds = |ctx: &ActionContext| -> Vec<ContactLogType> {
            ctx.commands
                .iter()
                .filter_map(|p| match &p.cmd {
                    Command::AddContactLog { kind, .. } => Some(*kind),
                    _ => None,
                })
                .collect()
        };

        // Contact ids are allocated in order, so one of these is even and the next is odd. Full
        // takes both; each parity takes exactly its own.
        let first_id = logged(&first)[0].0;
        let (even_ctx, odd_ctx) = if first_id.is_multiple_of(2) {
            (&first, &second)
        } else {
            (&second, &first)
        };

        assert!(kinds(even_ctx).contains(&ContactLogType::Full));
        assert!(kinds(even_ctx).contains(&ContactLogType::Even));
        assert!(!kinds(even_ctx).contains(&ContactLogType::Odd));

        assert!(kinds(odd_ctx).contains(&ContactLogType::Full));
        assert!(kinds(odd_ctx).contains(&ContactLogType::Odd));
        assert!(!kinds(odd_ctx).contains(&ContactLogType::Even));
    }

    // ---- effective possession ----

    // Watari owns the log; L reads it through an ActorLinkType::Passive link.
    #[test]
    fn a_link_grants_reach_into_the_log() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let watari = add_player(&mut eng, 0, Role::Watari, "watari");
        let l = add_player(&mut eng, 0, Role::L, "l");

        let members = members(&eng, the_log(&eng, ContactLogType::Full));

        assert!(members.contains(&watari), "the owner reads their own log");
        assert!(
            members.contains(&l),
            "and so does whoever is linked to them"
        );
    }

    // DisablePassiveLinks on the OWNER cuts the link, so the reader loses the log while the owner
    // keeps it.
    #[test]
    fn disabling_passive_links_revokes_the_reader() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let watari = add_player(&mut eng, 0, Role::Watari, "watari");
        let l = add_player(&mut eng, 0, Role::L, "l");

        // Death carries every modifier, DisablePassiveLinks among them.
        quick_kill(&mut eng, 1, false, false, false, watari);

        let members = members(&eng, the_log(&eng, ContactLogType::Full));

        assert!(!members.contains(&l));
    }

    #[test]
    fn an_unrelated_player_never_reaches_it() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        add_player(&mut eng, 0, Role::Watari, "watari");
        let stranger = add_player(&mut eng, 0, Role::Civilian, "stranger");

        let members = members(&eng, the_log(&eng, ContactLogType::Full));

        assert!(!members.contains(&stranger));
    }

    // The log is addressed to the world's Full record, so gaining the passive backfills everything
    // it ever recorded — the same rule channels follow.
    #[test]
    fn entries_are_addressed_to_the_world_log_viewport() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        add_player(&mut eng, 0, Role::Watari, "watari");
        let a = add_player(&mut eng, 0, Role::Civilian, "a");
        let b = add_player(&mut eng, 0, Role::Civilian, "b");
        let full = the_log(&eng, ContactLogType::Full);

        let ctx = basic_lounge(&mut eng, 1, a, b);

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(full)
                && matches!(&p.cmd, Command::AddContactLog { kind: ContactLogType::Full, .. })
        }));
    }

    // The bug this whole redesign exists for: a passive granted AFTER contacts were logged still
    // reaches them. The record lives on the world viewport, not the passive, so gaining the passive
    // only enters the actor into a viewport that already holds the full history — and entering
    // backfills it.
    #[test]
    fn a_passive_granted_late_still_reaches_earlier_contacts() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let a = add_player(&mut eng, 0, Role::Civilian, "a");
        let b = add_player(&mut eng, 0, Role::Civilian, "b");
        let latecomer = add_player(&mut eng, 0, Role::Civilian, "latecomer");

        // A contact happens before anyone holds the log. It still lands on the world record.
        let ctx = basic_lounge(&mut eng, 1, a, b);
        let full = the_log(&eng, ContactLogType::Full);
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(full)
                && matches!(&p.cmd, Command::AddContactLog { .. })
        }));
        assert!(
            !members(&eng, full).contains(&latecomer),
            "no route in yet"
        );

        // Only now does the latecomer gain a Full contact-log passive.
        quick_passive(
            &mut eng,
            2,
            latecomer,
            PassiveType::ContactLogs(ContactLogType::Full),
            false,
        );

        // Entering the record is what hands them the history it already holds.
        assert!(
            members(&eng, full).contains(&latecomer),
            "gaining the passive enters them into the record that predates it"
        );
    }
}

// test transfers
