pub mod create_incarceration;
pub mod cull_incarcerations;
pub mod release_incarceration;
pub mod update_prison_channel;

#[cfg(test)]
mod incarceration_tests {
    use lawliet_types::command::{Command, CommandRecipient};

    use crate::{
        actor::state::State,
        config::role::Role,
        engine::Engine,
        helpers::{get_actor, get_incarceration},
        test_helpers::{add_player, incarcerate, init_engine, release_incarceration},
    };

    fn world(eng: &mut Engine) -> (crate::ActorKey, crate::ActorKey) {
        init_engine(eng);
        let victim = add_player(eng, 0, Role::Civilian, "victim");
        let onlooker = add_player(eng, 0, Role::Civilian, "onlooker");
        (victim, onlooker)
    }

    fn announcement(ctx: &crate::action::ActionContext) -> Option<(Option<u128>, CommandRecipient)> {
        ctx.commands.iter().find_map(|p| match &p.cmd {
            Command::Incarceration { duration, .. } => Some((*duration, p.recipient.clone())),
            _ => None,
        })
    }

    #[test]
    fn incarcerating_announces_it_to_everyone_present() {
        let mut eng = Engine::new();
        let (victim, _) = world(&mut eng);
        let presence = eng.world.presence_viewport;

        let (_, ctx) = incarcerate(&mut eng, 1, victim, None);

        assert_eq!(
            announcement(&ctx),
            Some((None, CommandRecipient::Viewport(presence)))
        );
    }

    #[test]
    fn a_timed_incarceration_carries_its_duration() {
        let mut eng = Engine::new();
        let (victim, _) = world(&mut eng);

        let (_, ctx) = incarcerate(&mut eng, 1, victim, Some(5_000));

        assert_eq!(announcement(&ctx).map(|(d, _)| d), Some(Some(5_000)));
    }

    // The source is never disclosed, so nothing on the wire may name it.
    #[test]
    fn the_announcement_never_names_a_source() {
        let mut eng = Engine::new();
        let (victim, _) = world(&mut eng);

        let (_, ctx) = incarcerate(&mut eng, 1, victim, None);

        // Incarceration carries victim + duration only; there is no reveal command at all.
        assert!(!ctx.commands.iter().any(|p| matches!(
            &p.cmd,
            Command::KidnapReveal { .. }
        )));
    }

    // The victim has to hear it. Incarceration takes their presence, which removes them from the
    // viewport the announcement is addressed to — so announcing after the state change tells
    // everyone except the one person it happened to.
    #[test]
    fn the_victim_is_told_before_they_lose_presence() {
        let mut eng = Engine::new();
        let (victim, _) = world(&mut eng);

        let (_, ctx) = incarcerate(&mut eng, 1, victim, None);

        let announced = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::Incarceration { .. }))
            .expect("announced");
        let exited = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::ExitViewport { actor, .. } if *actor == victim))
            .expect("lost presence");

        assert!(
            announced < exited,
            "announced at {announced}, but the victim had already left at {exited}"
        );
    }

    #[test]
    fn a_duration_schedules_the_release() {
        let mut eng = Engine::new();
        let (victim, _) = world(&mut eng);

        let (id, _) = incarcerate(&mut eng, 1, victim, Some(5_000));
        assert!(get_incarceration(&eng, id).is_ok());

        // past the release
        crate::test_helpers::null_action(&mut eng, 7_000);

        assert!(get_incarceration(&eng, id).is_err());
        assert!(!get_actor(&eng, victim).unwrap().has_state(State::Incarcerated));
    }

    #[test]
    fn releasing_announces_it_and_frees_the_victim() {
        let mut eng = Engine::new();
        let (victim, _) = world(&mut eng);
        let presence = eng.world.presence_viewport;
        let (id, _) = incarcerate(&mut eng, 1, victim, None);

        let (_, ctx) = release_incarceration(&mut eng, 2, id).unwrap();

        assert!(ctx.commands.iter().any(|p| matches!(
            (&p.cmd, &p.recipient),
            (Command::IncarcerationReleased { .. }, CommandRecipient::Viewport(v)) if *v == presence
        )));
        assert!(!get_actor(&eng, victim).unwrap().has_state(State::Incarcerated));
    }
}
