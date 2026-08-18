pub mod create_orgs;
pub mod initialize_engine;
pub mod initialize_world;
pub mod next_iteration;
pub mod press_conf_access;
pub mod set_blackout;
pub mod set_news_anchor;
pub mod set_random_seed;
pub mod start_game;
pub mod update_actor_statuses;
pub mod update_contact_log_viewports;
pub mod update_org_effective_members;
pub mod update_press_conference;
pub mod update_world_viewports;

#[cfg(test)]
mod world_tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, Gun},
        action::{
            Action, ActionActor, ActionError, ActionRequest, ActionResponse, AddPlayer,
            CreateAndGiveAbility, LendNotebook,
        },
        command::{Command, CommandRecipient},
        world::WorldPhase,
    };

    use crate::{
        actor::state::State,
        channel::{ChannelPerm, ChannelPermSet},
        config::{ability::AbilityName, role::Role, world::WorldChannelName},
        engine::Engine,
        helpers::get_channel,
        test_helpers::*,
    };

    // Everything the player may do in a world channel, under any name they hold there.
    fn world_channel_perms(
        eng: &Engine,
        name: WorldChannelName,
        player_id: crate::common::ActorKey,
    ) -> ChannelPermSet {
        let channel_id = *eng.world.world_channel_map.get(&name).unwrap();
        get_channel(eng, channel_id)
            .unwrap()
            .owned_profiles(player_id)
            .fold(ChannelPermSet::EMPTY, |acc, profile| acc | profile.perms)
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
        let seat = join_channel(&mut eng, 0, p1, channel);
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
        }, Engine::version());
        assert!(matches!(lend, Err((ActionError::GameNotStarted, _))));
        assert!(
            send_message(&mut eng, 1, p1, channel, seat, "hello").is_ok(),
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

    // A blueprint is the whole of it: a world channel that has one seats everybody, and one that
    // does not seats nobody at all. There is no separate list of who belongs where.
    #[test]
    fn a_blueprint_is_what_puts_a_player_in_a_world_channel() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        for (name, channel_id) in eng
            .world
            .world_channel_map
            .iter()
            .map(|(name, id)| (*name, *id))
            .collect::<Vec<_>>()
        {
            let expected = eng.config.world_config.world_channels[&name].is_some();
            assert_eq!(
                get_channel(&eng, channel_id).unwrap().is_member(p1),
                expected,
                "{name:?}"
            );
        }
    }

    // The seat a player is handed is announced twice — to the room as a roster entry, and to the
    // player as their own access — and BOTH have to carry what it actually permits. The grant runs
    // before the policy that decides that, so an access telling its owner they hold nothing is a
    // seat they cannot use: the room can see them talk and they cannot find the send box.
    #[test]
    fn a_new_seat_tells_its_owner_what_it_permits() {
        let mut eng = Engine::new();
        init_engine(&mut eng);

        let (response, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::AddPlayer(AddPlayer {
                    true_name: "p1".into(),
                    starting_role: Role::Civilian,
                }),
            }, Engine::version())
            .unwrap();
        let ActionResponse::AddPlayer(data) = response else {
            unreachable!()
        };
        let p1 = data.id;
        let general = *eng
            .world
            .world_channel_map
            .get(&WorldChannelName::General)
            .unwrap();

        // The last word on each channel wins; anything earlier was superseded before the client
        // could act on it.
        let access = ctx
            .commands
            .iter()
            .rev()
            .find_map(|p| match (&p.cmd, &p.recipient) {
                (
                    Command::ProfileAccess {
                        channel_id,
                        profiles,
                    },
                    CommandRecipient::Actor(target),
                ) if *channel_id == general && *target == p1 => Some(profiles),
                _ => None,
            })
            .expect("a new player is told what they hold in the town square");

        assert!(
            access
                .iter()
                .any(|profile| profile.perms.contains(ChannelPerm::Send)),
            "own access says {access:?}, but the channel grants {:?}",
            world_channel_perms(&eng, WorldChannelName::General, p1)
        );
    }

    // ---- default permissions ----

    #[test]
    fn default_perms_no_modifiers() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let news_perms = world_channel_perms(&eng, WorldChannelName::News, p1);
        assert!(news_perms.contains(ChannelPerm::View));
        assert!(!news_perms.contains(ChannelPerm::Send));

        let gen_perms = world_channel_perms(&eng, WorldChannelName::General, p1);
        assert!(gen_perms.contains(ChannelPerm::Send));
        assert!(gen_perms.contains(ChannelPerm::View));
    }

    #[test]
    fn no_contact_removes_send_from_general() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(
            !world_channel_perms(&eng, WorldChannelName::General, p1).contains(ChannelPerm::Send)
        );
    }

    #[test]
    fn no_presence_removes_view_from_news() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(!world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::View));
    }

    #[test]
    fn state_removal_restores_perms() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        add_state(&mut eng, 0, p1, State::Dead);
        remove_state(&mut eng, 0, p1, State::Dead);

        let gen_perms = world_channel_perms(&eng, WorldChannelName::General, p1);
        assert!(gen_perms.contains(ChannelPerm::Send));
        assert!(gen_perms.contains(ChannelPerm::View));
    }

    // ---- the news anchor ----

    // Speaking on the news is a status, not a role: naming someone anchor hands them the NewsAccess
    // passive, which is what the news policy grants Send on.
    #[test]
    fn news_anchor_gets_send_on_news() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        assert!(
            !world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::Send),
            "not the anchor yet"
        );

        set_news_anchor(&mut eng, 0, Some(p1));

        assert!(world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::Send));
    }

    #[test]
    fn civilian_no_send_on_news() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        assert!(!world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::Send));
    }

    // Handing the post to someone else takes the old anchor's Send with it: the kit moves, it is
    // not copied.
    #[test]
    fn reassigning_the_anchor_clears_the_old_ones_send() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        set_news_anchor(&mut eng, 0, Some(p1));
        set_news_anchor(&mut eng, 0, Some(p2));

        assert!(!world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::Send));
        assert!(world_channel_perms(&eng, WorldChannelName::News, p2).contains(ChannelPerm::Send));
    }

    // Vacating the post strips the kit back to ownerless, so the last anchor can no longer broadcast.
    #[test]
    fn vacating_the_anchor_clears_send() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_news_anchor(&mut eng, 0, Some(p1));
        set_news_anchor(&mut eng, 0, None);

        assert!(!world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::Send));
    }

    // A guest in the press conference speaks on the news without being the anchor: press-conf
    // membership grants Send on its own.
    #[test]
    fn a_press_conference_guest_gets_send_on_news() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        assert!(!world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::Send));

        press_conf_access(&mut eng, 0, p1, true);

        assert!(world_channel_perms(&eng, WorldChannelName::News, p1).contains(ChannelPerm::Send));
    }

    // Losing presence drops a guest from the conference on the next sweep, and their news Send goes
    // with it — the host does not have to walk anyone out.
    #[test]
    fn losing_presence_evicts_a_press_conference_guest() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        press_conf_access(&mut eng, 0, p1, true);
        assert!(eng.world.news.press_conf.contains(&p1));

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(
            !eng.world.news.press_conf.contains(&p1),
            "a guest who cannot be present is not in the conference"
        );
    }

    // The anchor's own standing still gates it: NewsAccess grants Send, and being unable to be there
    // at all takes it back.
    #[test]
    fn a_dead_anchor_does_not_broadcast() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        set_news_anchor(&mut eng, 0, Some(p1));

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(world_channel_perms(&eng, WorldChannelName::News, p1).is_empty());
    }

    // L and Watari reach their channel by being what they are, and give the seat up when they stop
    // being it. The channel hands out nothing on its own.
    #[test]
    fn the_l_and_watari_line_follows_the_role() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let l = add_player(&mut eng, 0, Role::L, "l");
        let civilian = add_player(&mut eng, 0, Role::Civilian, "civilian");

        assert!(
            world_channel_perms(&eng, WorldChannelName::LAndWatari, l).contains(ChannelPerm::Send)
        );
        assert!(
            world_channel_perms(&eng, WorldChannelName::LAndWatari, civilian).is_empty(),
            "nobody else is even in it"
        );

        give_role(&mut eng, 1, l, Role::Civilian);

        assert!(world_channel_perms(&eng, WorldChannelName::LAndWatari, l).is_empty());
    }
}
