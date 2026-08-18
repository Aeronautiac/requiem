pub mod create_kidnapping;
pub mod cull_kidnappings;
pub mod release_kidnapping;
pub mod update_kidnappings;

#[cfg(test)]
mod tests {
    use lawliet_types::command::{Command, CommandRecipient};

    use crate::{
        action::{
            Action, ActionActor, ActionContext, ActionError, ActionRequest,
            ability::create_and_give_ability::CreateAndGiveAbility,
            kidnapping::create_kidnapping::CreateKidnapping,
            kidnapping::release_kidnapping::ReleaseKidnapping,
        },
        actor::{ActorDisplay, state::State},
        channel::{ChannelPerm, ChannelPermSet},
        common::{ActorKey, ChannelKey, KidnappingKey, Time},
        config::{ability::AbilityName, actor::organization::OrganizationName, role::Role},
        engine::Engine,
        helpers::{get_actor, get_channel, get_kidnapping},
        kidnapping::{KidnappingSource, KidnappingType},
        test_helpers::*,
    };

    // Everything somebody may do in the kidnapping's channel, under any name they hold there.
    // Nothing at all if they are not in it.
    fn perms(eng: &Engine, channel: ChannelKey, who: ActorKey) -> ChannelPermSet {
        get_channel(eng, channel)
            .unwrap()
            .owned_profiles(who)
            .fold(ChannelPermSet::EMPTY, |acc, profile| acc | profile.perms)
    }

    // The shared helper always kidnaps indefinitely; these cases need the duration and the
    // commands it produced.
    fn kidnap_for(
        eng: &mut Engine,
        time: Time,
        victim_id: ActorKey,
        duration: Option<Time>,
    ) -> (KidnappingKey, ActionContext) {
        let (data, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: time,
                payload: Action::CreateKidnapping(CreateKidnapping {
                    victim_id,
                    kidnapping_type: KidnappingType::Anonymous,
                    source: KidnappingSource::None,
                    duration,
                }),
            }, Engine::version())
            .unwrap();
        let crate::action::ActionResponse::CreateKidnapping(response) = data else {
            unreachable!()
        };
        (response.id, ctx)
    }

    // Basic creation: victim gets Kidnapped state and Send | View on the channel.
    #[test]
    fn create_sets_kidnapped_state_and_channel_perms() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (_, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::None,
        );

        assert!(get_actor(&eng, victim).unwrap().has_state(State::Kidnapped));
        assert!(perms(&eng, ch, victim).contains(ChannelPerm::Send));
        assert!(perms(&eng, ch, victim).contains(ChannelPerm::View));
    }

    // Victim death drops their channel perms to EMPTY; the kidnapping itself persists.
    #[test]
    fn victim_perms_drop_on_death() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (kid_id, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::None,
        );
        quick_kill(&mut eng, 1, false, false, false, victim);

        assert_eq!(perms(&eng, ch, victim), ChannelPermSet::EMPTY);
        assert!(get_kidnapping(&eng, kid_id).is_ok());
    }

    // Reviving the victim restores Send | View.
    #[test]
    fn victim_perms_restore_on_revive() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (_, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::None,
        );
        quick_kill(&mut eng, 1, false, false, false, victim);
        quick_revive(&mut eng, 2, true, victim);

        assert!(perms(&eng, ch, victim).contains(ChannelPerm::Send));
        assert!(perms(&eng, ch, victim).contains(ChannelPerm::View));
    }

    // Killing the ability owner leaves the kidnapping and victim perms fully intact.
    #[test]
    fn ability_owner_death_preserves_kidnapping() {
        let mut eng = Engine::new();
        let owner = add_player(&mut eng, 0, Role::Civilian, "owner");
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::AnonymousKidnap,
                variant: 0,
                actor_id: owner,
                volatile: false,
                transferrable: false,
            },
        );

        let (kid_id, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::Ability(ab),
        );
        quick_kill(&mut eng, 1, false, false, false, owner);

        assert!(get_kidnapping(&eng, kid_id).is_ok());
        assert!(perms(&eng, ch, victim).contains(ChannelPerm::Send));
        assert!(perms(&eng, ch, victim).contains(ChannelPerm::View));
    }

    // When an org member (kidnapper side) dies, their channel perms drop to EMPTY.
    #[test]
    fn org_member_perms_drop_on_death() {
        let mut eng = Engine::new();
        let org = add_org(&mut eng, 0, OrganizationName::NULL);
        let member = add_player(&mut eng, 0, Role::Civilian, "member");
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        add_to_org(&mut eng, 0, org, member, false, true).unwrap();

        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::AnonymousKidnap,
                variant: 0,
                actor_id: org,
                volatile: false,
                transferrable: false,
            },
        );

        let (_, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::Ability(ab),
        );

        assert!(perms(&eng, ch, member).contains(ChannelPerm::Send));

        quick_kill(&mut eng, 1, false, false, false, member);

        // Dying takes them off the org's channel, and the sweep reads participation from there —
        // so they stop being a captor entirely rather than sitting in the room with nothing.
        assert_eq!(perms(&eng, ch, member), ChannelPermSet::EMPTY);
    }

    // Anonymous kidnapping: org members appear as Mysterious.
    #[test]
    fn anon_org_member_displays_as_mysterious() {
        let mut eng = Engine::new();
        let org = add_org(&mut eng, 0, OrganizationName::NULL);
        let member = add_player(&mut eng, 0, Role::Civilian, "member");
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        add_to_org(&mut eng, 0, org, member, false, true).unwrap();

        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::AnonymousKidnap,
                variant: 0,
                actor_id: org,
                volatile: false,
                transferrable: false,
            },
        );

        let (_, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::Ability(ab),
        );

        // One mask between them, and nothing of their own: the victim cannot tell one captor from
        // another, or how many there are.
        let names = get_channel(&eng, ch).unwrap().accessible_profiles(member);
        assert!(names.iter().any(|p| p.display == ActorDisplay::Mysterious));
        assert!(!names.iter().any(|p| p.display == ActorDisplay::Raw(member)));
    }

    // Public kidnapping: org members appear as Raw(member_id).
    #[test]
    fn public_org_member_displays_as_raw() {
        let mut eng = Engine::new();
        let org = add_org(&mut eng, 0, OrganizationName::NULL);
        let member = add_player(&mut eng, 0, Role::Civilian, "member");
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        add_to_org(&mut eng, 0, org, member, false, true).unwrap();

        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::PublicKidnap,
                variant: 0,
                actor_id: org,
                volatile: false,
                transferrable: false,
            },
        );

        let (_, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Public(ActorDisplay::Raw(member)),
            KidnappingSource::Ability(ab),
        );

        // A name of their own, and it is the announced face, so the room sees it from the start.
        let names = get_channel(&eng, ch).unwrap().accessible_profiles(member);
        assert!(names.iter().any(|p| p.display == ActorDisplay::Raw(member)));
        assert!(!names.iter().any(|p| p.display == ActorDisplay::Mysterious));
        assert!(
            get_channel(&eng, ch)
                .unwrap()
                .visible_profiles()
                .iter()
                .any(|p| p.display == ActorDisplay::Raw(member))
        );
    }

    // Release removes Kidnapped state, destroys the channel, and removes the kidnapping record.
    #[test]
    fn release_cleans_up_state_and_channel() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (kid_id, ch) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::None,
        );
        release_kidnapping(&mut eng, 1, kid_id);

        assert!(!get_actor(&eng, victim).unwrap().has_state(State::Kidnapped));
        assert!(get_kidnapping(&eng, kid_id).is_err());
        assert!(get_channel(&eng, ch).is_err());
    }

    // A third-party player cannot release a kidnapping they don't own the source ability for.
    #[test]
    fn release_by_unrelated_player_is_rejected() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        let other = add_player(&mut eng, 0, Role::Civilian, "other");

        let (kid_id, _) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::None,
        );

        let result = eng.execute(ActionRequest {
            actor: ActionActor::Player(other),
            timestamp: 1,
            payload: Action::ReleaseKidnapping(ReleaseKidnapping {
                kidnapping_id: kid_id,
                forced: false,
            }),
        }, Engine::version());
        assert!(matches!(
            result,
            Err((ActionError::InsufficientPermissions, _))
        ));
    }

    // The ability owner can release their own kidnapping.
    #[test]
    fn ability_owner_can_release() {
        let mut eng = Engine::new();
        let owner = add_player(&mut eng, 0, Role::Civilian, "owner");
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::AnonymousKidnap,
                variant: 0,
                actor_id: owner,
                volatile: false,
                transferrable: false,
            },
        );

        let (kid_id, _) = create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::Ability(ab),
        );

        let result = eng.execute(ActionRequest {
            actor: ActionActor::Player(owner),
            timestamp: 1,
            payload: Action::ReleaseKidnapping(ReleaseKidnapping {
                kidnapping_id: kid_id,
                forced: false,
            }),
        }, Engine::version());
        assert!(result.is_ok());
        assert!(get_kidnapping(&eng, kid_id).is_err());
    }

    // Dead players have NoPresence and cannot be kidnapped.
    #[test]
    fn cannot_kidnap_dead_player() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        quick_kill(&mut eng, 0, false, false, false, victim);

        let result = eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 1,
            payload: Action::CreateKidnapping(
                crate::action::kidnapping::create_kidnapping::CreateKidnapping {
                    victim_id: victim,
                    kidnapping_type: KidnappingType::Anonymous,
                    source: KidnappingSource::None,
                    duration: None,
                },
            ),
        }, Engine::version());
        assert!(matches!(result, Err((ActionError::UserNotPresent, _))));
    }

    // Already-kidnapped players have NoPresence and cannot be kidnapped again.
    #[test]
    fn cannot_kidnap_already_kidnapped_player() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        create_kidnapping(
            &mut eng,
            0,
            victim,
            KidnappingType::Anonymous,
            KidnappingSource::None,
        );

        let result = eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 1,
            payload: Action::CreateKidnapping(
                crate::action::kidnapping::create_kidnapping::CreateKidnapping {
                    victim_id: victim,
                    kidnapping_type: KidnappingType::Anonymous,
                    source: KidnappingSource::None,
                    duration: None,
                },
            ),
        }, Engine::version());
        assert!(matches!(result, Err((ActionError::UserNotPresent, _))));
    }

    // IPP players have StrengthenedPresence and cannot be kidnapped.
    #[test]
    fn cannot_kidnap_ipp_player() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        add_state(&mut eng, 0, victim, State::Ipp);

        let result = eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 1,
            payload: Action::CreateKidnapping(
                crate::action::kidnapping::create_kidnapping::CreateKidnapping {
                    victim_id: victim,
                    kidnapping_type: KidnappingType::Anonymous,
                    source: KidnappingSource::None,
                    duration: None,
                },
            ),
        }, Engine::version());
        assert!(matches!(
            result,
            Err((ActionError::ActorHasStrengthenedPresence, _))
        ));
    }

    #[test]
    fn kidnapping_announces_it_to_everyone_present_with_its_duration() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");
        let events = eng.world.events_viewport;

        let (_, ctx) = kidnap_for(&mut eng, 1, victim, Some(5_000));

        assert_eq!(
            ctx.commands.iter().find_map(|p| match &p.cmd {
                Command::Kidnapping { duration, .. } => Some((*duration, p.recipient.clone())),
                _ => None,
            }),
            Some((Some(5_000), CommandRecipient::Viewport(events)))
        );
    }

    // Kidnapped carries NoPresence, which takes the victim out of the viewport the announcement
    // is addressed to — announcing afterwards tells everyone except the person it happened to.
    #[test]
    fn the_victim_is_told_before_they_lose_presence() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (_, ctx) = kidnap_for(&mut eng, 1, victim, None);

        let announced = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::Kidnapping { .. }))
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
        init_engine(&mut eng);
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (id, _) = kidnap_for(&mut eng, 1, victim, Some(5_000));
        assert!(get_kidnapping(&eng, id).is_ok());

        // past the release
        null_action(&mut eng, 7_000);

        assert!(get_kidnapping(&eng, id).is_err());
        assert!(!get_actor(&eng, victim).unwrap().has_state(State::Kidnapped));
    }

    #[test]
    fn no_duration_never_releases_on_its_own() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (id, _) = kidnap_for(&mut eng, 1, victim, None);
        null_action(&mut eng, 100_000);

        assert!(get_kidnapping(&eng, id).is_ok());
        assert!(get_actor(&eng, victim).unwrap().has_state(State::Kidnapped));
    }
}
