pub mod add_state;
pub mod create_actor_links;
pub mod org;
pub mod player;
pub mod purge_volatiles;
pub mod remove_state;
pub mod sever_links;

#[cfg(test)]
mod state_tests {
    use lawliet_types::{
        actor::{State, States},
        command::{Command, CommandRecipient},
    };

    use crate::{
        action::ActionContext,
        common::ActorKey,
        config::role::Role,
        engine::Engine,
        test_helpers::{add_player, add_state, init_engine, remove_state},
    };

    // Every state set this actor was told about, in the order they were sent.
    fn told(ctx: &ActionContext, actor: ActorKey) -> Vec<States> {
        ctx.commands
            .iter()
            .filter_map(|p| match &p.cmd {
                Command::ActorState { state, actor_id }
                    if *actor_id == actor && p.recipient == CommandRecipient::Actor(actor) =>
                {
                    Some(*state)
                }
                _ => None,
            })
            .collect()
    }

    fn player(eng: &mut Engine) -> ActorKey {
        init_engine(eng);
        add_player(eng, 0, Role::Civilian, "subject")
    }

    // The WHOLE set, not the state that just changed — a client that ever missed one update would
    // otherwise stay wrong forever.
    #[test]
    fn an_actor_is_told_their_whole_state_set() {
        let mut eng = Engine::new();
        let subject = player(&mut eng);

        let (_, first) = add_state(&mut eng, 1, subject, State::Ipp);
        assert_eq!(told(&first, subject), vec![States::from(State::Ipp)]);

        let (_, second) = add_state(&mut eng, 2, subject, State::Custody);
        assert_eq!(told(&second, subject), vec![State::Ipp | State::Custody]);
    }

    #[test]
    fn removing_a_state_reports_what_is_left() {
        let mut eng = Engine::new();
        let subject = player(&mut eng);
        add_state(&mut eng, 1, subject, State::Ipp);
        add_state(&mut eng, 2, subject, State::Custody);

        let (_, ctx) = remove_state(&mut eng, 3, subject, State::Ipp);

        assert_eq!(told(&ctx, subject), vec![States::from(State::Custody)]);
    }

    // Adding a state an actor already holds changes nothing, and saying so again is noise.
    #[test]
    fn a_redundant_change_says_nothing() {
        let mut eng = Engine::new();
        let subject = player(&mut eng);
        add_state(&mut eng, 1, subject, State::Ipp);

        let (_, repeat) = add_state(&mut eng, 2, subject, State::Ipp);
        assert!(told(&repeat, subject).is_empty());

        let (_, absent) = remove_state(&mut eng, 3, subject, State::Kidnapped);
        assert!(told(&absent, subject).is_empty());
    }

    // The reason this is directed rather than addressed to a viewport: State::Dead carries
    // NoPresence, so by the time the death is announced the subject is out of the viewport carrying
    // it. They still have to learn they are dead.
    #[test]
    fn a_dying_actor_still_hears_it_after_losing_presence() {
        let mut eng = Engine::new();
        let subject = player(&mut eng);

        let (_, ctx) = add_state(&mut eng, 1, subject, State::Dead);

        assert_eq!(told(&ctx, subject), vec![States::from(State::Dead)]);
        let stated = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::ActorState { .. }))
            .expect("the subject was told");
        let exited = ctx
            .commands
            .iter()
            .position(
                |p| matches!(&p.cmd, Command::ExitViewport { actor, .. } if *actor == subject),
            )
            .expect("and then lost presence");
        assert!(
            stated < exited,
            "told at {stated}, but had already left at {exited}"
        );
    }
}
