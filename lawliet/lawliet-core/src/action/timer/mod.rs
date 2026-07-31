pub mod update_timers;

#[cfg(test)]
mod timer_tests {
    use crate::{
        ActorKey, PollKey,
        action::poll::create_poll::CreatePoll,
        actor::state::State,
        channel::{ChannelPerm, ChannelPermSet},
        config::{actor::organization::OrganizationName, role::Role},
        engine::Engine,
        helpers::get_actor,
        poll::{PollOption, PollParent, PollPolicy, PollSubject, VoterPolicy},
        test_helpers::*,
    };

    // A vote put to the world that kills its victim when it times out.
    fn timed_poll(eng: &mut Engine, victim: ActorKey) -> PollKey {
        create_poll(
            eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(100),
                options: PollOption::accept_reject(Some(default_kill(victim)), None),
                ignore_amplification: false,
            },
        )
    }

    fn dead(eng: &Engine, id: ActorKey) -> bool {
        get_actor(eng, id).unwrap().has_state(State::Dead)
    }

    // The whole point: time spent in the dark is given back rather than merely survived. The poll
    // had 60 left when the lights went out, so it has 60 left when they come back — not 60 minus
    // the 560 the world spent dark.
    #[test]
    fn a_blackout_gives_back_the_time_it_took() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let poll = timed_poll(&mut eng, p1);
        add_vote(&mut eng, 0, poll, p1, ACCEPT).unwrap();

        set_blackout(&mut eng, 40, true);
        // Well past the original deadline of 100.
        null_action(&mut eng, 600);
        assert!(!dead(&eng, p1));

        set_blackout(&mut eng, 600, false);
        null_action(&mut eng, 659);
        assert!(!dead(&eng, p1));

        null_action(&mut eng, 660);
        assert!(dead(&eng, p1));
    }

    // The other side of the derivation: an org poll's clock cannot be stopped, because an org poll
    // is seen through org membership and a blackout does not touch that. Its members can still
    // watch it, still vote on it, and it keeps its original deadline.
    #[test]
    fn an_org_poll_runs_out_in_the_dark() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let org = add_org(&mut eng, 0, OrganizationName::NULL);
        add_to_org(&mut eng, 0, org, p1, false, true).unwrap();

        let poll = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::Org(org),
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(100),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        set_blackout(&mut eng, 40, true);
        add_vote(&mut eng, 50, poll, p1, ACCEPT).unwrap();
        null_action(&mut eng, 100);

        assert!(dead(&eng, p1));
    }

    // The other half of that: a poll put to the world is not merely paused in the dark, it is
    // unreachable. Nobody can see it, so nobody may enter a vote on it.
    #[test]
    fn a_world_poll_cannot_be_voted_on_in_the_dark() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let poll = timed_poll(&mut eng, p1);

        set_blackout(&mut eng, 40, true);
        assert!(add_vote(&mut eng, 50, poll, p1, ACCEPT).is_err());

        set_blackout(&mut eng, 60, false);
        add_vote(&mut eng, 70, poll, p1, ACCEPT).unwrap();
    }

    // A channel poll is seen by whoever can READ the channel, not by whoever is merely in it.
    // Revoking View locks a member out of the ballot without discarding the vote they already
    // entered — the same shape the world case has, reached through the same viewport mirroring.
    #[test]
    fn a_channel_poll_follows_the_channels_sight() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let channel = create_channel(&mut eng, 0, false);
        let seat = join_channel(&mut eng, 0, p1, channel);

        let poll = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::Channel(channel),
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(None, None),
                ignore_amplification: false,
            },
        );
        add_vote(&mut eng, 0, poll, p1, ACCEPT).unwrap();

        // Still a member, and still counted — but with nothing to read, there is no ballot to
        // reach.
        set_profile_perms(&mut eng, 1, channel, seat, ChannelPermSet::EMPTY).unwrap();

        let poll_data = eng.world.get_poll(poll).unwrap();
        assert!(poll_data.counts(&eng, p1));
        assert!(!poll_data.can_enter(&eng, p1));
        assert!(poll_data.contains_voter(p1));
        assert!(remove_vote(&mut eng, 2, poll, p1).is_err());
    }

    // Counting is not entering. A vote cast in the light keeps counting once the world goes dark,
    // so the world can still reach a conclusion on what it already holds — here a death does it,
    // by removing the only voter standing between one vote and a majority.
    #[test]
    fn votes_already_cast_still_count_in_the_dark() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: Some(100),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        // One vote of two possible is not a majority.
        add_vote(&mut eng, 0, poll, p1, ACCEPT).unwrap();
        assert!(!dead(&eng, p1));

        set_blackout(&mut eng, 10, true);
        // One vote of one is. The tally that carries it was entered before the lights went out,
        // and nothing in the dark discarded it.
        quick_kill(&mut eng, 20, true, true, false, p2);

        assert!(dead(&eng, p1));
    }

    // The channel case, which the old world-only derivation could not answer: a ballot nobody in
    // the channel can read must not be running out either. Nothing here mentions channels — the
    // timer is gated on the viewport its poll rides, and that viewport emptying is the whole
    // signal.
    #[test]
    fn a_channel_poll_stops_when_nobody_can_read_it() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let channel = create_channel(&mut eng, 0, false);
        let seat = join_channel(&mut eng, 0, p1, channel);

        let poll = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::Channel(channel),
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(100),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );
        add_vote(&mut eng, 0, poll, p1, ACCEPT).unwrap();
        assert!(eng.world.timers.values().all(|timer| !timer.is_paused()));

        set_profile_perms(&mut eng, 10, channel, seat, ChannelPermSet::EMPTY).unwrap();
        assert!(eng.world.timers.values().all(|timer| timer.is_paused()));

        // Well past the original deadline, and it has not run out.
        null_action(&mut eng, 500);
        assert!(!dead(&eng, p1));

        set_profile_perms(
            &mut eng,
            500,
            channel,
            seat,
            ChannelPerm::View | ChannelPerm::Send,
        )
        .unwrap();
        // 10 of the 100 were served before the lights went out; the other 90 are still owed.
        null_action(&mut eng, 589);
        assert!(!dead(&eng, p1));
        null_action(&mut eng, 590);
        assert!(dead(&eng, p1));
    }

    // A timer started while the world is already dark never gets a head start: the sweep that
    // trails the creating action stops it on the way out.
    #[test]
    fn a_timer_started_in_the_dark_starts_paused() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        set_blackout(&mut eng, 0, true);
        timed_poll(&mut eng, p1);

        assert!(eng.world.timers.values().all(|timer| timer.is_paused()));

        null_action(&mut eng, 500);
        assert!(!dead(&eng, p1));
    }

    // Freeing a poll takes its timer with it. Without that, the timeout would fire into the gap
    // the poll left behind.
    #[test]
    fn resolving_a_poll_frees_its_timer() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: Some(100),
                options: PollOption::accept_reject(Some(default_kill(p2)), None),
                ignore_amplification: false,
            },
        );
        assert_eq!(eng.world.timers.len(), 1);

        // Majority on the update policy resolves it immediately, well inside its duration.
        add_vote(&mut eng, 0, poll, p1, ACCEPT).unwrap();
        add_vote(&mut eng, 0, poll, p2, ACCEPT).unwrap();

        assert!(eng.world.polls.is_empty());
        assert!(eng.world.timers.is_empty());
    }
}
