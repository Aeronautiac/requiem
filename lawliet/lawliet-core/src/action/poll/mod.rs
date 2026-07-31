pub mod add_vote;
pub mod create_poll;
pub mod poll_cleanup;
pub mod poll_timeout;
pub mod remove_vote;
pub mod update_polls;

use lawliet_types::command::{Command, CommandRecipient};

use crate::{
    action::ActionContext,
    common::PollKey,
    engine::Engine,
    helpers::get_poll,
    poll::{PollOptionIndex, PollOptionTally},
};

// Broadcast a poll's current state: the shared data + tally to the parent's viewport
// (UpdatePoll), then each viewer's personal view — votability and their own vote
// (UpdatePollView). Only emits on the mutate pass; used on creation and after each vote.
//
// Nothing here decides who may read any of it. The poll is addressed to its parent's viewport
// and the parent alone decides that viewport's membership, so a player who has lost sight of the
// poll simply stops receiving updates and keeps whatever it last looked like to them.
pub(crate) fn broadcast_poll(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    poll_id: PollKey,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    let Ok(poll) = get_poll(eng, poll_id) else {
        return;
    };
    let Some(viewport) = poll.viewport(eng) else {
        return;
    };

    let viewers = poll.viewers(eng);
    let tally = poll.weights(eng);
    let options = poll
        .options
        .iter()
        .zip(tally.options)
        .map(|(option, weight)| PollOptionTally {
            label: option.label.clone(),
            weight,
        })
        .collect();
    let subject = poll.subject.clone();
    let parent = poll.parent;
    let opener = poll.opener;
    ctx.push_cmd(
        Command::UpdatePoll {
            poll_id,
            subject,
            parent,
            options,
            potential: tally.potential_total,
            opener,
        },
        CommandRecipient::Viewport(viewport),
        eng.time,
    );

    // Personal views stay directed: `eligible` and `own_vote` differ per viewer, so they cannot
    // ride the shared viewport.
    for id in viewers {
        let poll = get_poll(eng, poll_id).expect("just read above");
        let eligible = poll.can_enter(eng, id);
        let own_vote: Option<PollOptionIndex> = poll.votes.get(&id).copied();
        ctx.push_cmd(
            Command::UpdatePollView {
                poll_id,
                eligible,
                own_vote,
            },
            CommandRecipient::Actor(id),
            eng.time,
        );
    }
}

// - Polls should cancel themselves if the action attached to them is rejected (pass mutate false)
// - The poll create action will check the attached action as well to gate initial creation
// - Adding a vote to a poll should first evaluate the poll
// - There should be an update action which handles sub-actions like poll
// updates which should run to keep game state up to date for things that may seem unrelated but
// significantly effect world state. Just scheduling poll update actions every 10 seconds or so is
// both unfair and inefficient. For example, an actor may die pushing the game state into a place
// where a poll may pass and imprison someone, but since it didnt update, that person may be able to
// do something before they were imprisoned even though they should already be in prison
// - An update action should ALWAYS run after any other action (things like polls may
// change depending on the things that other actions do. for example, killing a member of kira's
// kingdom who voted no for a poll might push that poll into the passing threshold even though there
// was no direct update to the poll
// - On every update, polls should be evaluated and checked for validity
// - Additionally, polls should be updated when they are interacted with (it is not
// necessary to even call the update function directly in handlers which simply modify poll state
// because the update action will be called directly afterwards anyway)
// - Update actions are called only AFTER other actions because there can be no poll with no initial
// creation action, and padding both sides would lead to double updates between every event
// (pointless)

// Update actions should be called not in the engine, but in the action execute function
// Dry runs SHOULD NOT call poll updates, only execute actions
// Interleaving is not an issue because actions are atomic by nature

// these tests will largely just use polls for killing people as that is a very easy action to test
// the polls will all have different configurations and voting scenarios ranging from actors with
// vote amplification passives, dead voters, side effect based executions, etc...
#[cfg(test)]
mod poll_tests {
    use crate::{
        action::{Action, engine::null::Null, poll::create_poll::CreatePoll},
        actor::state::State,
        config::role::Role,
        engine::Engine,
        helpers::get_actor,
        passive::PassiveType,
        poll::{PollOption, PollOptionLabel, PollParent, PollPolicy, PollSubject, VoterPolicy},
        test_helpers::*,
    };

    fn generic_option(label: &str, payload: Option<Action>) -> PollOption {
        PollOption {
            label: PollOptionLabel::Generic(label.to_string()),
            payload,
        }
    }

    #[test]
    fn vote_addition() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        // will never resolve
        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(Some(Action::Null(Null {})), None),
                ignore_amplification: false,
            },
        );

        let poll_data = eng.world.get_poll(poll_id).unwrap();
        assert!(!poll_data.contains_voter(p1));

        add_vote(&mut eng, 0, poll_id, p1, ACCEPT).unwrap();

        let poll_data = eng.world.get_poll(poll_id).unwrap();
        assert!(poll_data.contains_voter(p1));
    }

    #[test]
    fn vote_addition_already_voted() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        // will never resolve
        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(Some(Action::Null(Null {})), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, ACCEPT).unwrap();
        assert!(add_vote(&mut eng, 0, poll_id, p1, ACCEPT).is_err());

        let poll_data = eng.world.get_poll(poll_id).unwrap();
        assert!(poll_data.contains_voter(p1));
    }

    #[test]
    fn vote_removal() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        // will never resolve
        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(Some(Action::Null(Null {})), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, ACCEPT).unwrap();
        remove_vote(&mut eng, 0, poll_id, p1).unwrap();

        let poll_data = eng.world.get_poll(poll_id).unwrap();
        assert!(!poll_data.contains_voter(p1));
    }

    #[test]
    fn vote_removal_hasnt_voted() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(None, None),
                ignore_amplification: false,
            },
        );

        let poll_data = eng.world.get_poll(poll_id).unwrap();
        assert!(!poll_data.contains_voter(p1));

        assert!(remove_vote(&mut eng, 0, poll_id, p1).is_err());

        let poll_data = eng.world.get_poll(poll_id).unwrap();
        assert!(!poll_data.contains_voter(p1));
    }

    #[test]
    fn vote_removal_doesnt_pass_voter_policy() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(None, None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, ACCEPT).unwrap();
        quick_kill(&mut eng, 0, true, true, false, p1);
        assert!(remove_vote(&mut eng, 0, poll_id, p1).is_err());
    }

    #[test]
    fn vote_addition_doesnt_pass_voter_policy() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(None, None),
                ignore_amplification: false,
            },
        );

        quick_kill(&mut eng, 0, true, true, false, p1);
        assert!(add_vote(&mut eng, 0, poll_id, p1, ACCEPT).is_err());
    }

    #[test]
    fn present_majority_update_majority_timeout() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::Majority,
                duration: Some(20),
                options: PollOption::accept_reject(Some(default_kill(p2)), None),
                ignore_amplification: false,
            },
        );

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p2_actor.has_state(State::Dead));

        add_vote(&mut eng, 1, poll_id, p1, ACCEPT).unwrap();

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p2_actor.has_state(State::Dead));

        null_action(&mut eng, 20);

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p2_actor.has_state(State::Dead));
    }

    #[test]
    fn simultaneous_timeout() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: Some(10),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        assert!(add_vote(&mut eng, 10, poll_id, p1, ACCEPT).is_err());
    }

    #[test]
    fn present_majority_update_no_timeout() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(Some(default_kill(p2)), None),
                ignore_amplification: false,
            },
        );

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p2_actor.has_state(State::Dead));

        add_vote(&mut eng, 1, poll_id, p1, ACCEPT).unwrap();

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p2_actor.has_state(State::Dead));

        add_vote(&mut eng, 21, poll_id, p2, ACCEPT).unwrap();

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(p2_actor.has_state(State::Dead));
    }

    #[test]
    fn present_majority_update_winning_timeout() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p3 = add_player(&mut eng, 0, Role::Civilian, "p3");
        let p4 = add_player(&mut eng, 0, Role::Civilian, "p4");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(10),
                options: PollOption::accept_reject(Some(default_kill(p4)), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 1, poll_id, p1, ACCEPT).unwrap();

        let p4_actor = get_actor(&eng, p4).unwrap();
        assert!(!p4_actor.has_state(State::Dead));

        add_vote(&mut eng, 9, poll_id, p2, ACCEPT).unwrap();

        let p4_actor = get_actor(&eng, p4).unwrap();
        assert!(!p4_actor.has_state(State::Dead));

        add_vote(&mut eng, 9, poll_id, p3, REJECT).unwrap();

        let p4_actor = get_actor(&eng, p4).unwrap();
        assert!(!p4_actor.has_state(State::Dead));

        // now winning vote should evaluate on timeout. since 2 > 1, it should kill p4.
        null_action(&mut eng, 20);

        let p4_actor = get_actor(&eng, p4).unwrap();
        assert!(p4_actor.has_state(State::Dead));
    }

    #[test]
    fn present_no_update_winning_timeout() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(10),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 1, poll_id, p1, ACCEPT).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(!p1_actor.has_state(State::Dead));

        null_action(&mut eng, 20);

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(p1_actor.has_state(State::Dead));
    }

    #[test]
    fn present_no_update_majority_timeout() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p3 = add_player(&mut eng, 0, Role::Civilian, "p3");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::Majority,
                duration: Some(10),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 1, poll_id, p1, ACCEPT).unwrap();
        add_vote(&mut eng, 1, poll_id, p2, ACCEPT).unwrap();
        add_vote(&mut eng, 1, poll_id, p3, ACCEPT).unwrap();

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(!p1_actor.has_state(State::Dead));

        null_action(&mut eng, 20);

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(p1_actor.has_state(State::Dead));
    }

    #[test]
    fn present_timeout_majority_failure() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p3 = add_player(&mut eng, 0, Role::Civilian, "p3");

        quick_passive(
            &mut eng,
            0,
            p3,
            PassiveType::VoteAmplification { multiplier: 2 },
            false,
        );

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::Majority,
                duration: Some(5),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, ACCEPT).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, ACCEPT).unwrap();
        add_vote(&mut eng, 0, poll_id, p3, REJECT).unwrap();
        null_action(&mut eng, 10);

        // it should be a 50/50 split
        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
    }

    #[test]
    fn vote_amplification() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        quick_passive(
            &mut eng,
            0,
            p2,
            PassiveType::VoteAmplification { multiplier: 10 },
            false,
        );

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::Majority,
                duration: Some(5),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, REJECT).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, ACCEPT).unwrap();
        null_action(&mut eng, 10);

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(p1_actor.has_state(State::Dead));
    }

    // test the scenario where a death allows a vote to cross threshold
    #[test]
    fn voter_death_majority_update() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::Majority,
                duration: None,
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, ACCEPT).unwrap();
        quick_kill(&mut eng, 0, true, true, false, p2);

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(p1_actor.has_state(State::Dead));
    }

    #[test]
    fn voter_death_winning_vote_timeout() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        quick_passive(
            &mut eng,
            0,
            p2,
            PassiveType::VoteAmplification { multiplier: 10 },
            false,
        );

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(5),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, ACCEPT).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, REJECT).unwrap();
        quick_kill(&mut eng, 0, true, true, false, p2);
        null_action(&mut eng, 10);

        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(p1_actor.has_state(State::Dead));
    }

    #[test]
    fn action_becomes_invalid() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(Some(default_kill(p2)), None),
                ignore_amplification: false,
            },
        );

        quick_kill(&mut eng, 0, true, true, false, p2);
        assert!(add_vote(&mut eng, 0, poll_id, p1, ACCEPT).is_err());
    }

    #[test]
    fn rejection_payload() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(Some(default_kill(p1)), Some(default_kill(p2))),
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, REJECT).unwrap();

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(!p2_actor.has_state(State::Dead));

        add_vote(&mut eng, 0, poll_id, p2, REJECT).unwrap();

        let p2_actor = get_actor(&eng, p2).unwrap();
        assert!(p2_actor.has_state(State::Dead));
        let p1_actor = get_actor(&eng, p1).unwrap();
        assert!(!p1_actor.has_state(State::Dead));
    }

    // Three options, no majority anywhere, and the heaviest one wins on timeout.
    #[test]
    fn most_voted_picks_the_heaviest_of_three() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p3 = add_player(&mut eng, 0, Role::Civilian, "p3");
        let p4 = add_player(&mut eng, 0, Role::Civilian, "p4");
        let p5 = add_player(&mut eng, 0, Role::Civilian, "p5");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(10),
                options: vec![
                    generic_option("a", None),
                    generic_option("b", Some(default_kill(p1))),
                    generic_option("c", None),
                ],
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, 0).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, 1).unwrap();
        add_vote(&mut eng, 0, poll_id, p3, 1).unwrap();
        add_vote(&mut eng, 0, poll_id, p4, 2).unwrap();

        // 1-2-1 of a possible 5. No majority, and p5 never voted, so only MostVoted can call it.
        assert!(!get_actor(&eng, p1).unwrap().has_state(State::Dead));
        assert!(!get_actor(&eng, p5).unwrap().has_state(State::Dead));

        null_action(&mut eng, 20);
        assert!(get_actor(&eng, p1).unwrap().has_state(State::Dead));
    }

    // A level split resolves to nobody, which is what keeps a two-way tie from silently going to
    // whichever option happens to be listed first.
    #[test]
    fn most_voted_is_inconclusive_on_a_tie() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::MostVoted,
                duration: Some(10),
                options: vec![
                    generic_option("a", Some(default_kill(p1))),
                    generic_option("b", Some(default_kill(p2))),
                ],
                ignore_amplification: false,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, 0).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, 1).unwrap();
        null_action(&mut eng, 20);

        assert!(!get_actor(&eng, p1).unwrap().has_state(State::Dead));
        assert!(!get_actor(&eng, p2).unwrap().has_state(State::Dead));
    }

    // An option that is not on the ballot is not a vote.
    #[test]
    fn a_vote_for_no_option_is_rejected() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: None,
                options: PollOption::accept_reject(None, None),
                ignore_amplification: false,
            },
        );

        assert!(add_vote(&mut eng, 0, poll_id, p1, 2).is_err());
        assert!(!eng.world.get_poll(poll_id).unwrap().contains_voter(p1));
    }

    // The same ballot that vote_amplification carries, with amplification switched off: one voter
    // worth ten is worth one, so the other side is not outvoted.
    #[test]
    fn ignoring_amplification_counts_heads() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        quick_passive(
            &mut eng,
            0,
            p2,
            PassiveType::VoteAmplification { multiplier: 10 },
            false,
        );

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::World,
                update_policy: PollPolicy::Majority,
                timeout_policy: PollPolicy::Majority,
                duration: Some(5),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: true,
            },
        );

        add_vote(&mut eng, 0, poll_id, p1, REJECT).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, ACCEPT).unwrap();
        null_action(&mut eng, 10);

        assert!(!get_actor(&eng, p1).unwrap().has_state(State::Dead));
    }

    // A poll cannot outlive what it was put to. The channel is torn down, so the vote it was held
    // in has no audience and no reason to still be open.
    #[test]
    fn destroying_a_parent_cancels_its_poll() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let channel = create_channel(&mut eng, 0, false);

        let poll_id = create_poll(
            &mut eng,
            0,
            CreatePoll {
                opener: None,
                voter_policy: VoterPolicy::Present,
                subject: PollSubject::Generic(String::new()),
                parent: PollParent::Channel(channel),
                update_policy: PollPolicy::AlwaysInconclusive,
                timeout_policy: PollPolicy::AlwaysInconclusive,
                duration: Some(100),
                options: PollOption::accept_reject(Some(default_kill(p1)), None),
                ignore_amplification: false,
            },
        );
        assert!(eng.world.get_poll(poll_id).is_some());

        destroy_channel(&mut eng, 1, channel).unwrap();

        assert!(eng.world.get_poll(poll_id).is_none());
        // And its timer went with it, rather than firing into the gap it left.
        assert!(eng.world.timers.is_empty());
    }
}
