pub mod add_passive;
pub mod create_and_give_passive;
pub mod destroy_passive;
pub mod give_passive;
pub mod update_passive_visibilities;

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
        common::{ID, PassiveKey, ViewportKey},
        config::role::Role,
        engine::Engine,
        lounge::LoungeVariant,
        passive::PassiveType,
        test_helpers::{add_player, init_engine, quick_kill},
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
        })
        .unwrap()
        .1
    }

    // The one contact-log passive in the world, and its viewport.
    fn the_log(eng: &Engine) -> (PassiveKey, ViewportKey) {
        eng.world
            .passives
            .iter()
            .find_map(|(id, p)| {
                matches!(p.passive_type, PassiveType::ContactLogs(_)).then_some((id, p.viewport))
            })
            .expect("a contact log passive")
    }

    fn logged(ctx: &ActionContext) -> Vec<(ID, ActorDisplay, ActorDisplay, ContactEvent)> {
        ctx.commands
            .iter()
            .filter_map(|p| match &p.cmd {
                Command::AddContactLog { log, .. } => {
                    Some((log.contact_id, log.contactor, log.contacted, log.event))
                }
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
            })
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
        add_player(&mut eng, 0, Role::Near, "near");
        add_player(&mut eng, 0, Role::Mello, "mello");
        let a = add_player(&mut eng, 0, Role::Civilian, "a");
        let b = add_player(&mut eng, 0, Role::Civilian, "b");

        let near_log = eng
            .world
            .passives
            .iter()
            .find_map(|(id, p)| {
                matches!(
                    p.passive_type,
                    PassiveType::ContactLogs(ContactLogType::Even)
                )
                .then_some(id)
            })
            .expect("near's log");
        let odd_log = eng
            .world
            .passives
            .iter()
            .find_map(|(id, p)| {
                matches!(
                    p.passive_type,
                    PassiveType::ContactLogs(ContactLogType::Odd)
                )
                .then_some(id)
            })
            .expect("mello's log");

        let first = basic_lounge(&mut eng, 1, a, b);
        let second = basic_lounge(&mut eng, 2, b, a);

        let receivers = |ctx: &ActionContext| -> Vec<PassiveKey> {
            ctx.commands
                .iter()
                .filter_map(|p| match &p.cmd {
                    Command::AddContactLog { passive_id, .. } => Some(*passive_id),
                    _ => None,
                })
                .collect()
        };

        // Contact ids are allocated in order, so one of these is even and the next is odd.
        let first_id = logged(&first)[0].0;
        let (even_ctx, odd_ctx) = if first_id.is_multiple_of(2) {
            (&first, &second)
        } else {
            (&second, &first)
        };

        assert!(receivers(even_ctx).contains(&near_log));
        assert!(!receivers(even_ctx).contains(&odd_log));
        assert!(receivers(odd_ctx).contains(&odd_log));
        assert!(!receivers(odd_ctx).contains(&near_log));
    }

    // ---- effective possession ----

    // Watari owns the log; L reads it through an ActorLinkType::Passive link.
    #[test]
    fn a_link_grants_reach_into_the_log() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let watari = add_player(&mut eng, 0, Role::Watari, "watari");
        let l = add_player(&mut eng, 0, Role::L, "l");

        let (_, viewport) = the_log(&eng);
        let members: Vec<ActorKey> = eng
            .world
            .get_viewport(viewport)
            .expect("log viewport")
            .members()
            .collect();

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

        let (_, viewport) = the_log(&eng);
        let members: Vec<ActorKey> = eng
            .world
            .get_viewport(viewport)
            .expect("log viewport")
            .members()
            .collect();

        assert!(!members.contains(&l));
    }

    #[test]
    fn an_unrelated_player_never_reaches_it() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        add_player(&mut eng, 0, Role::Watari, "watari");
        let stranger = add_player(&mut eng, 0, Role::Civilian, "stranger");

        let (_, viewport) = the_log(&eng);
        let members: Vec<ActorKey> = eng
            .world
            .get_viewport(viewport)
            .expect("log viewport")
            .members()
            .collect();

        assert!(!members.contains(&stranger));
    }

    // The log is addressed to the passive's viewport, so gaining the passive backfills everything
    // it ever recorded — the same rule channels follow.
    #[test]
    fn entries_are_addressed_to_the_passives_viewport() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        add_player(&mut eng, 0, Role::Watari, "watari");
        let a = add_player(&mut eng, 0, Role::Civilian, "a");
        let b = add_player(&mut eng, 0, Role::Civilian, "b");
        let (passive_id, viewport) = the_log(&eng);

        let ctx = basic_lounge(&mut eng, 1, a, b);

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::AddContactLog { passive_id: id, .. } if *id == passive_id)
        }));
    }
}

// test transfers
