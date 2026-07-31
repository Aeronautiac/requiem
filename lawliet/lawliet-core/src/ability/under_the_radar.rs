// Take yourself off the record for the rest of the iteration. State::UnderTheRadar carries
// Modifier::LogNullification, which SendMessage reads to suppress both the entry it would write to
// the user's log viewport and every bug relay watching them — so nothing they say is recoverable
// by an autopsy or heard by a tap.
//
// Iteration-scoped, like Ipp: NextIteration clears it, so there is no expiry to track here.

use lawliet_types::{
    ability::{AbilityName, UnderTheRadar},
    action::{Action, ActionActor, AddState},
    actor::State,
};

use crate::{
    ability::AbilityInterface,
    action::ActionInterface,
    helpers::{actor_id, get_player},
};

impl AbilityInterface for UnderTheRadar {
    fn ability_name(&self) -> AbilityName {
        AbilityName::UnderTheRadar
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        _ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.player_only()?;
        let user_id = actor_id(actor).expect("expected valid actor to use UnderTheRadar");
        get_player(eng, user_id)?;

        Action::AddState(AddState {
            actor_id: user_id,
            state: State::UnderTheRadar,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, UnderTheRadar},
        action::CreateAndGiveAbility,
        actor::State,
        command::{Command, CommandRecipient},
    };

    use crate::{
        action::ActionContext,
        common::{ActorKey, ChannelKey, LogID, ProfileKey, ViewportKey},
        config::role::Role,
        engine::Engine,
        helpers::{get_actor, get_channel, get_player},
        test_helpers::{
            add_player, create_channel, init_engine, join_channel, next_iteration, quick_ability,
            send_message, use_ability,
        },
    };

    // A user holding the ability, and a loggable channel they can speak in.
    fn world(eng: &mut Engine) -> (ActorKey, ChannelKey, ProfileKey, crate::AbilityKey) {
        init_engine(eng);
        let user = add_player(eng, 0, Role::Civilian, "user");
        let channel = create_channel(eng, 0, true);
        let profile = join_channel(eng, 0, user, channel);
        let ability = quick_ability(
            eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::UnderTheRadar,
                variant: 0,
                actor_id: user,
                volatile: false,
                transferrable: false,
            },
        );
        (user, channel, profile, ability)
    }

    fn player_log(eng: &Engine, user: ActorKey) -> LogID {
        get_player(eng, user).unwrap().log
    }

    fn logged_to(ctx: &ActionContext, log: LogID) -> usize {
        ctx.commands
            .iter()
            .filter(|p| p.recipient == CommandRecipient::Log(log))
            .count()
    }

    fn addressed_to(ctx: &ActionContext, viewport: ViewportKey) -> usize {
        ctx.commands
            .iter()
            .filter(|p| p.recipient == CommandRecipient::Viewport(viewport))
            .count()
    }

    // An enabled bug on the user. Custody-sourced so it needs no owning ability; who reads it is
    // irrelevant here, only that it exists to relay through.
    fn bug_on(eng: &mut Engine, time: crate::Time, target: ActorKey) -> ViewportKey {
        let data = eng
            .execute(crate::action::ActionRequest {
                actor: crate::action::ActionActor::System,
                timestamp: time,
                payload: crate::action::Action::CreateBug(crate::action::CreateBug {
                    target_id: target,
                    source: crate::bug::BugSource::Custody,
                }),
            })
            .unwrap()
            .0;
        let crate::action::ActionResponse::CreateBug(response) = data else {
            unreachable!()
        };
        crate::helpers::get_bug(eng, response.id).unwrap().viewport
    }

    fn basic_lounge(
        eng: &mut Engine,
        time: crate::Time,
        contactor: ActorKey,
        contacted: ActorKey,
    ) -> ActionContext {
        eng.execute(crate::action::ActionRequest {
            actor: crate::action::ActionActor::System,
            timestamp: time,
            payload: crate::action::Action::CreateLounge(crate::action::CreateLounge {
                variant: crate::lounge::LoungeVariant::Basic {
                    contactor_id: contactor,
                    contacted_id: contacted,
                },
            }),
        })
        .unwrap()
        .1
    }

    // The channel always hears it; only the record stops.
    #[test]
    fn speaking_normally_writes_to_the_senders_log() {
        let mut eng = Engine::new();
        let (user, channel, profile, _) = world(&mut eng);
        let log = player_log(&eng, user);

        let (_, ctx) = send_message(&mut eng, 1, user, channel, profile, "on the record").unwrap();

        assert_eq!(logged_to(&ctx, log), 1);
    }

    #[test]
    fn under_the_radar_leaves_no_log_entry() {
        let mut eng = Engine::new();
        let (user, channel, profile, ability) = world(&mut eng);
        let log = player_log(&eng, user);

        use_ability(
            &mut eng,
            1,
            user,
            ability,
            AbilityBehaviour::UnderTheRadar(UnderTheRadar {}),
        )
        .unwrap();

        let (_, ctx) = send_message(&mut eng, 2, user, channel, profile, "off the record").unwrap();

        assert_eq!(logged_to(&ctx, log), 0);
        // ...but the room still heard it.
        let channel_viewport = get_channel(&eng, channel).unwrap().viewport;
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(channel_viewport)
                && matches!(&p.cmd, Command::AddMessage { .. })
        }));
    }

    // The channel's record is separate from its live delivery precisely so these can disagree. A
    // tap-in reads the record, so an unlogged message must not be sitting in it — while the room
    // that actually heard it keeps it.
    #[test]
    fn the_channels_record_omits_it_too() {
        let mut eng = Engine::new();
        let (user, channel, profile, ability) = world(&mut eng);
        let channel_log = get_channel(&eng, channel).unwrap().log;

        let (_, before) =
            send_message(&mut eng, 1, user, channel, profile, "on the record").unwrap();
        assert_eq!(logged_to(&before, channel_log), 1);

        use_ability(
            &mut eng,
            2,
            user,
            ability,
            AbilityBehaviour::UnderTheRadar(UnderTheRadar {}),
        )
        .unwrap();

        let (_, after) =
            send_message(&mut eng, 3, user, channel, profile, "off the record").unwrap();
        assert_eq!(logged_to(&after, channel_log), 0);
    }

    #[test]
    fn the_state_carries_log_nullification() {
        let mut eng = Engine::new();
        let (user, _, _, ability) = world(&mut eng);

        use_ability(
            &mut eng,
            1,
            user,
            ability,
            AbilityBehaviour::UnderTheRadar(UnderTheRadar {}),
        )
        .unwrap();

        let actor = get_actor(&eng, user).unwrap();
        assert!(actor.has_state(State::UnderTheRadar));
        assert!(actor.has_modifier(crate::actor::modifier::Modifier::LogNullification));
    }

    // The other two surfaces LogNullification covers: a bug watching the user relays nothing, and
    // the contacts they make leave no line in anybody's contact log.
    #[test]
    fn a_bug_watching_them_relays_nothing() {
        let mut eng = Engine::new();
        let (user, channel, profile, ability) = world(&mut eng);
        let bug_viewport = bug_on(&mut eng, 1, user);

        use_ability(
            &mut eng,
            2,
            user,
            ability,
            AbilityBehaviour::UnderTheRadar(UnderTheRadar {}),
        )
        .unwrap();

        let (_, ctx) = send_message(&mut eng, 3, user, channel, profile, "off the record").unwrap();

        assert_eq!(addressed_to(&ctx, bug_viewport), 0);
    }

    #[test]
    fn their_contacts_are_not_logged() {
        let mut eng = Engine::new();
        let (user, _, _, ability) = world(&mut eng);
        let other = add_player(&mut eng, 0, Role::Civilian, "other");
        // A contact-log passive exists to receive it, if anything were written.
        // Watari owns the ContactLogs passive; L only reaches it through a link.
        add_player(&mut eng, 0, Role::Watari, "watcher");

        use_ability(
            &mut eng,
            1,
            user,
            ability,
            AbilityBehaviour::UnderTheRadar(UnderTheRadar {}),
        )
        .unwrap();

        let ctx = basic_lounge(&mut eng, 2, user, other);

        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::AddContactLog { .. }))
        );
    }

    // The same contact, made by someone on the record, does land — so the test above is proving
    // nullification rather than an absent log.
    #[test]
    fn an_ordinary_contact_is_logged() {
        let mut eng = Engine::new();
        let (user, _, _, _) = world(&mut eng);
        let other = add_player(&mut eng, 0, Role::Civilian, "other");
        // Watari owns the ContactLogs passive; L only reaches it through a link.
        add_player(&mut eng, 0, Role::Watari, "watcher");

        let ctx = basic_lounge(&mut eng, 1, user, other);

        assert!(
            ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::AddContactLog { .. }))
        );
    }

    // Iteration-scoped: the boundary is what ends it, not a timer.
    #[test]
    fn it_lasts_until_the_next_iteration() {
        let mut eng = Engine::new();
        let (user, channel, profile, ability) = world(&mut eng);
        let log = player_log(&eng, user);
        use_ability(
            &mut eng,
            1,
            user,
            ability,
            AbilityBehaviour::UnderTheRadar(UnderTheRadar {}),
        )
        .unwrap();

        next_iteration(&mut eng, 2);

        assert!(
            !get_actor(&eng, user)
                .unwrap()
                .has_state(State::UnderTheRadar)
        );
        let (_, ctx) =
            send_message(&mut eng, 3, user, channel, profile, "back on the record").unwrap();
        assert_eq!(logged_to(&ctx, log), 1);
    }
}
