pub mod add_to_world_channels;
pub mod create_orgs;
pub mod initialize_engine;
pub mod initialize_world;
pub mod next_iteration;
pub mod set_random_seed;
pub mod start_game;
pub mod set_world_channel_override;
pub mod update_world_channel_perms;

#[cfg(test)]
mod world_tests {
    use indexmap::indexset;
    use lawliet_types::{
        ability::{AbilityBehaviour, Gun},
        action::{
            Action, ActionActor, ActionError, ActionRequest, CreateAndGiveAbility, LendNotebook,
        },
        actor::ActorDisplay,
        world::WorldPhase,
    };

    use crate::{
        actor::{
            player::{OverrideResolver, OverrideSource, WorldChannelOverride},
            state::State,
        },
        channel::{ChannelMember, ChannelPermission, ChannelPermissions},
        config::{ability::AbilityName, role::Role, world::WorldChannelName},
        engine::Engine,
        helpers::{get_channel, get_player},
        test_helpers::*,
    };

    fn world_channel_perms(
        eng: &Engine,
        name: WorldChannelName,
        player_id: crate::common::ActorKey,
    ) -> ChannelPermissions {
        let channel_id = *eng.world.world_channel_map.get(&name).unwrap();
        get_channel(eng, channel_id)
            .unwrap()
            .get_member(player_id)
            .unwrap()
            .perms
    }

    // ---- phases ----

    // A world in setup is a real, populated place. What waits for the start is play: abilities, and
    // using or passing a notebook. Talking is not play — ordinary channel permission answers that.
    #[test]
    fn setup_blocks_play_but_not_talking() {
        let mut eng = Engine::new();
        init_engine_unstarted(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let channel = create_channel(&mut eng, 0, true);
        set_member(
            &mut eng,
            0,
            p1,
            channel,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();
        let ability = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                actor_id: p1,
                volatile: false,
                transferrable: false,
            },
        );
        let notebook = quick_notebook(&mut eng, 0, p1, false);

        assert!(matches!(
            use_ability(
                &mut eng,
                1,
                p1,
                ability,
                AbilityBehaviour::Gun(Gun { target_id: p2 })
            ),
            Err((ActionError::GameNotStarted, _))
        ));
        // Not quick_lend, which unwraps and so cannot express a rejection.
        let lend = eng.execute(ActionRequest {
            actor: ActionActor::Player(p1),
            timestamp: 1,
            payload: Action::LendNotebook(LendNotebook {
                notebook_id: notebook,
                target_id: p2,
            }),
        });
        assert!(matches!(lend, Err((ActionError::GameNotStarted, _))));
        assert!(
            send_message(&mut eng, 1, p1, channel, ActorDisplay::Raw(p1), "hello").is_ok(),
            "talking is not play"
        );

        start_game(&mut eng, 2).unwrap();
        assert!(
            use_ability(
                &mut eng,
                3,
                p1,
                ability,
                AbilityBehaviour::Gun(Gun { target_id: p2 })
            )
            .is_ok()
        );
    }

    // The start IS the first turn of the clock, so the world opens on iteration 1 — iteration 0
    // being the time before play.
    #[test]
    fn starting_turns_the_first_day() {
        let mut eng = Engine::new();
        init_engine_unstarted(&mut eng);
        assert_eq!(eng.world.phase, WorldPhase::Setup);
        assert_eq!(eng.world.curr_iteration, 0);

        start_game(&mut eng, 0).unwrap();

        assert_eq!(eng.world.phase, WorldPhase::Running);
        assert_eq!(eng.world.curr_iteration, 1);
        assert!(matches!(
            start_game(&mut eng, 1),
            Err((ActionError::GameAlreadyStarted, _))
        ));
    }

    // Autonomous days re-arm themselves; an early manual turn cancels the pending one so the new
    // day gets a full duration rather than the remainder of the last.
    #[test]
    fn an_early_turn_restarts_the_clock() {
        let mut eng = Engine::new();
        init_engine_unstarted(&mut eng);
        start_game(&mut eng, 0).unwrap();

        let armed = eng.world.iteration_job.expect("autonomous days arm a job");
        assert_eq!(
            eng.jobs.view(armed).unwrap().request.timestamp,
            eng.config.defaults.iteration_duration
        );

        let early = 5_000;
        next_iteration(&mut eng, early);

        let rearmed = eng.world.iteration_job.expect("and re-arm on every turn");
        assert_ne!(rearmed, armed);
        assert_eq!(
            eng.jobs.view(rearmed).unwrap().request.timestamp,
            early + eng.config.defaults.iteration_duration
        );
    }

    // The queue runs on the way IN to an action, so one action long after the fact catches the
    // world all the way up rather than losing the days it slept through — and rather than turning
    // one day and leaving the rest pending. This is what makes an idle game survive a gap, since
    // nothing else drives the clock: no action, no time.
    #[test]
    fn one_late_action_turns_every_day_it_passed() {
        let mut eng = Engine::new();
        init_engine_unstarted(&mut eng);
        start_game(&mut eng, 0).unwrap();
        assert_eq!(eng.world.curr_iteration, 1);

        // A Null carries no intent of its own; all it does is drag the queue along with it.
        let day = eng.config.defaults.iteration_duration;
        null_action(&mut eng, day * 3 + 1);

        assert_eq!(eng.world.curr_iteration, 4);
        // And the clock is still armed, from the last day it turned rather than from where it
        // started — a catch-up leaves the world running, not merely correct.
        let armed = eng.world.iteration_job.expect("still armed");
        assert_eq!(
            eng.jobs.view(armed).unwrap().request.timestamp,
            day * 3 + day
        );
    }

    // Handing the clock to the host means nothing is ever scheduled; the day turns when they say.
    #[test]
    fn a_host_owned_clock_arms_nothing() {
        let mut eng = Engine::new();
        init_engine_unstarted(&mut eng);
        eng.config.defaults.iterations_autonomous = false;

        start_game(&mut eng, 0).unwrap();

        assert_eq!(eng.world.curr_iteration, 1);
        assert!(eng.world.iteration_job.is_none());
    }

    // ---- initialization ----

    #[test]
    fn init_creates_world_channels() {
        let mut eng = Engine::new();
        init_engine(&mut eng);

        assert!(
            eng.world
                .world_channel_map
                .contains_key(&WorldChannelName::News)
        );
        assert!(
            eng.world
                .world_channel_map
                .contains_key(&WorldChannelName::General)
        );
    }

    #[test]
    fn world_channels_are_loggable() {
        let mut eng = Engine::new();
        init_engine(&mut eng);

        for channel_id in eng
            .world
            .world_channel_map
            .values()
            .copied()
            .collect::<Vec<_>>()
        {
            assert!(get_channel(&eng, channel_id).unwrap().loggable);
        }
    }

    // ---- membership ----

    #[test]
    fn player_added_to_all_world_channels() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        for channel_id in eng
            .world
            .world_channel_map
            .values()
            .copied()
            .collect::<Vec<_>>()
        {
            assert!(
                get_channel(&eng, channel_id)
                    .unwrap()
                    .get_member(p1)
                    .is_some()
            );
        }
    }

    // ---- default permissions ----

    #[test]
    fn default_perms_no_modifiers() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let news_perms = world_channel_perms(&eng, WorldChannelName::News, p1);
        assert!(news_perms.contains(ChannelPermission::View));
        assert!(!news_perms.contains(ChannelPermission::Send));

        let gen_perms = world_channel_perms(&eng, WorldChannelName::General, p1);
        assert!(gen_perms.contains(ChannelPermission::Send));
        assert!(gen_perms.contains(ChannelPermission::View));
    }

    #[test]
    fn no_contact_removes_send_from_general() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(
            !world_channel_perms(&eng, WorldChannelName::General, p1)
                .contains(ChannelPermission::Send)
        );
    }

    #[test]
    fn no_presence_removes_view_from_news() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(
            !world_channel_perms(&eng, WorldChannelName::News, p1)
                .contains(ChannelPermission::View)
        );
    }

    #[test]
    fn state_removal_restores_perms() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        add_state(&mut eng, 0, p1, State::Dead);
        remove_state(&mut eng, 0, p1, State::Dead);

        let gen_perms = world_channel_perms(&eng, WorldChannelName::General, p1);
        assert!(gen_perms.contains(ChannelPermission::Send));
        assert!(gen_perms.contains(ChannelPermission::View));
    }

    // ---- overrides ----

    #[test]
    fn default_override_replaces_world_default() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::Send | ChannelPermission::View,
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();

        assert!(
            world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPermission::Send)
        );
    }

    #[test]
    fn default_override_still_blocked_by_modifiers() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::Send | ChannelPermission::View,
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();
        add_state(&mut eng, 0, p1, State::Dead);

        assert!(world_channel_perms(&eng, WorldChannelName::News, p1).is_empty());
    }

    #[test]
    fn force_override_bypasses_blocking() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            Some(WorldChannelOverride {
                default_perms: ChannelPermissions::EMPTY,
                force_perms: ChannelPermission::Send | ChannelPermission::View,
            }),
        )
        .unwrap();
        add_state(&mut eng, 0, p1, State::Dead);

        let perms = world_channel_perms(&eng, WorldChannelName::News, p1);
        assert!(perms.contains(ChannelPermission::Send));
        assert!(perms.contains(ChannelPermission::View));
    }

    #[test]
    fn clearing_override_reverts_to_world_default() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::Send | ChannelPermission::View,
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();
        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            None,
        )
        .unwrap();

        let perms = world_channel_perms(&eng, WorldChannelName::News, p1);
        assert!(perms.contains(ChannelPermission::View));
        assert!(!perms.contains(ChannelPermission::Send));
    }

    // ---- role overrides ----

    #[test]
    fn news_anchor_gets_send_on_news() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::NewsAnchor, "p1");

        assert!(
            world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPermission::Send)
        );
    }

    #[test]
    fn civilian_no_send_on_news() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        assert!(
            !world_channel_perms(&eng, WorldChannelName::News, p1)
                .contains(ChannelPermission::Send)
        );
    }

    #[test]
    fn role_change_clears_news_send() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::NewsAnchor, "p1");

        give_role(&mut eng, 0, p1, Role::Civilian);

        assert!(
            !world_channel_perms(&eng, WorldChannelName::News, p1)
                .contains(ChannelPermission::Send)
        );
    }

    #[test]
    fn role_override_blocked_by_modifiers() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::NewsAnchor, "p1");

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(
            !world_channel_perms(&eng, WorldChannelName::News, p1)
                .contains(ChannelPermission::Send)
        );
    }

    // clearing a force override while blocking modifiers are active exposes the blocked state
    #[test]
    fn clearing_force_override_exposes_blocking_state() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            Some(WorldChannelOverride {
                default_perms: ChannelPermissions::EMPTY,
                force_perms: ChannelPermission::Send | ChannelPermission::View,
            }),
        )
        .unwrap();
        add_state(&mut eng, 0, p1, State::Dead);

        // force is active, blocking state has no effect
        assert!(!world_channel_perms(&eng, WorldChannelName::News, p1).is_empty());

        // clearing the override exposes the blocking state
        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            None,
        )
        .unwrap();
        assert!(world_channel_perms(&eng, WorldChannelName::News, p1).is_empty());
    }

    // higher-priority source wins over lower-priority source
    #[test]
    fn higher_priority_override_wins() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            0,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::View.into(),
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(1),
            1,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::Send | ChannelPermission::View,
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();

        assert!(
            world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPermission::Send)
        );
    }

    // equal-priority tie with positive resolver: on wins
    #[test]
    fn tied_priority_positive_resolver_grants_send() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            1,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::View.into(),
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(1),
            1,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::Send | ChannelPermission::View,
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();

        // positive resolver (used by UpdateWorldChannelPerms): send wins
        assert!(
            world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPermission::Send)
        );
    }

    // equal-priority tie with negative resolver: all must agree, so send is absent
    #[test]
    fn tied_priority_negative_resolver_requires_consensus() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(0),
            1,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::Send | ChannelPermission::View,
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();

        set_world_channel_override(
            &mut eng,
            0,
            p1,
            WorldChannelName::News,
            OverrideSource::Manual(1),
            1,
            Some(WorldChannelOverride {
                default_perms: ChannelPermission::View.into(),
                force_perms: ChannelPermissions::EMPTY,
            }),
        )
        .unwrap();

        let result = get_player(&eng, p1)
            .unwrap()
            .get_world_channel_override(WorldChannelName::News, OverrideResolver::Negative)
            .unwrap();

        assert!(!result.default_perms.contains(ChannelPermission::Send));
        assert!(result.default_perms.contains(ChannelPermission::View));
    }
}
