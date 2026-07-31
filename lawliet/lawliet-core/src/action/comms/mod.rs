pub mod bug;
pub mod channel;
pub mod groupchat;
pub mod lounge;

#[cfg(test)]
mod comms_tests {
    use lawliet_types::{channel::FixedPolicy, command::CommandRecipient};

    use crate::{
        action::{
            Action, ActionActor, ActionError, ActionRequest, ActionResponse,
            ability::{add_ability::AddAbility, create_and_give_ability::CreateAndGiveAbility},
            actor::add_state::AddState,
            comms::{
                bug::{archive_bug::ArchiveBug, create_bug::CreateBug, destroy_bug::DestroyBug},
                channel::{
                    create_and_give_profile::CreateAndGiveProfile, send_message::SendMessage,
                    set_loggable::SetLoggable, set_profile_access::SetProfileAccess,
                },
                groupchat::create_groupchat::CreateGroupchat,
                lounge::create_lounge::CreateLounge,
            },
        },
        actor::{ActorDisplay, state::State},
        bug::BugSource,
        channel::{ChannelKind, ChannelPerm, ChannelPermSet, PermUpdatePolicy},
        command::Command,
        common::{AbilityKey, ActorKey, BugKey, ChannelKey, ViewportKey},
        config::{ability::AbilityName, role::Role},
        engine::Engine,
        helpers::{get_bug, get_channel, get_gc, get_player},
        lounge::LoungeVariant,
        passive::PassiveType,
        test_helpers::*,
    };

    // ---- channel ----

    // Everything an actor may do in a channel, under any name they hold there.
    fn perms(eng: &Engine, channel: ChannelKey, who: ActorKey) -> ChannelPermSet {
        get_channel(eng, channel)
            .unwrap()
            .owned_profiles(who)
            .fold(ChannelPermSet::EMPTY, |acc, profile| acc | profile.perms)
    }

    // Membership is not a thing granted on its own. Holding a name here IS being in the channel,
    // and there is no being in one under no name at all.
    #[test]
    fn a_name_is_what_makes_you_a_member() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        assert!(!get_channel(&eng, ch).unwrap().is_member(p1));

        join_channel(&mut eng, 0, p1, ch);

        assert!(get_channel(&eng, ch).unwrap().is_member(p1));
        assert!(perms(&eng, ch, p1).contains(ChannelPerm::Send));
    }

    #[test]
    fn losing_your_last_name_ends_the_membership() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        join_channel(&mut eng, 0, p1, ch);

        remove_from_channel(&mut eng, 0, p1, ch).unwrap();

        assert!(!get_channel(&eng, ch).unwrap().is_member(p1));
    }

    // A name is announced twice over, to two different audiences answering two different
    // questions: the room is shown what it can see, and the holder is told what is theirs to use.
    #[test]
    fn a_granted_name_reaches_the_room_and_its_holder() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateAndGiveProfile(CreateAndGiveProfile {
                    channel_id: ch,
                    player_id: p1,
                    display: ActorDisplay::Raw(p1),
                    visible: true,
                    shared: false,
                    transferrable: false,
                    perm_policy: PermUpdatePolicy::Fixed(FixedPolicy {
                        perms: ChannelPerm::Send | ChannelPerm::View,
                    }),
                }),
            })
            .unwrap();

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(&p.cmd, Command::ProfileAccess { channel_id, profiles }
                    if *channel_id == ch
                        && profiles.iter().any(|v| v.display == ActorDisplay::Raw(p1)))
        }));
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(&p.cmd, Command::ChannelRoster { channel_id, profiles }
                    if *channel_id == ch
                        && profiles.iter().any(|v| v.display == ActorDisplay::Raw(p1)))
        }));
    }

    // The roster is current state rather than a sequence of events, so it is addressed to each
    // viewer by name. Addressed to the viewport it would be replayed to every later arrival, and
    // every name the channel ever held would come with it.
    #[test]
    fn the_roster_is_never_addressed_to_the_viewport() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateAndGiveProfile(CreateAndGiveProfile {
                    channel_id: ch,
                    player_id: p1,
                    display: ActorDisplay::Raw(p1),
                    visible: true,
                    shared: false,
                    transferrable: false,
                    perm_policy: PermUpdatePolicy::Fixed(FixedPolicy {
                        perms: ChannelPerm::Send | ChannelPerm::View,
                    }),
                }),
            })
            .unwrap();

        assert!(!ctx.commands.iter().any(|p| {
            matches!(&p.cmd, Command::ChannelRoster { .. })
                && matches!(p.recipient, CommandRecipient::Viewport(_))
        }));
    }

    // A name the room has not been told about is genuinely absent from the roster. Its existence
    // is the thing being kept, so an anonymous holder is not exposed by the shape of the traffic.
    #[test]
    fn an_invisible_name_is_off_the_roster_until_it_speaks() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let ch = create_channel(&mut eng, 0, false);
        join_channel(&mut eng, 0, p2, ch);

        let (response, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateAndGiveProfile(CreateAndGiveProfile {
                    channel_id: ch,
                    player_id: p1,
                    display: ActorDisplay::Mysterious,
                    visible: false,
                    shared: false,
                    transferrable: false,
                    perm_policy: PermUpdatePolicy::Fixed(FixedPolicy {
                        perms: ChannelPerm::Send | ChannelPerm::View,
                    }),
                }),
            })
            .unwrap();
        let ActionResponse::CreateAndGiveProfile(data) = response else {
            unreachable!()
        };

        // p1 is told about their own name — that is what they will speak as. p2 is not.
        assert!(!ctx.commands.iter().any(|p| {
            matches!(&p.cmd, Command::ChannelRoster { profiles, .. }
                if profiles.iter().any(|v| v.display == ActorDisplay::Mysterious))
        }));
        assert!(
            !get_channel(&eng, ch)
                .unwrap()
                .visible_profiles()
                .iter()
                .any(|v| v.display == ActorDisplay::Mysterious)
        );

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, data.profile_id, "hello").unwrap();

        // Revealed by the message, and revealed BEFORE it, so nothing is attributed to a name the
        // room has not been given yet.
        let roster = ctx
            .commands
            .iter()
            .position(|p| {
                matches!(&p.cmd, Command::ChannelRoster { profiles, .. }
                    if profiles.iter().any(|v| v.display == ActorDisplay::Mysterious))
            })
            .expect("sending reveals the name");
        let message = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::AddMessage { .. }))
            .expect("the message itself");
        assert!(roster < message);
    }

    // A name only one person can be wearing is the whole reason wearing it means anything, so
    // handing it to a second one is refused rather than quietly ignored.
    #[test]
    fn an_exclusive_name_is_refused_to_a_second_holder() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let ch = create_channel(&mut eng, 0, false);
        let seat = join_channel(&mut eng, 0, p1, ch);

        let result = eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::SetProfileAccess(SetProfileAccess {
                channel_id: ch,
                profile_id: seat,
                player_id: p2,
                granted: true,
            }),
        });

        assert!(matches!(
            result,
            Err((ActionError::ProfileNotShareable, _))
        ));
    }

    // A name that could never have belonged to anybody else has no life without its holder, so
    // leaving destroys it. One that could is merely taken off them and left behind.
    #[test]
    fn leaving_destroys_only_the_names_bound_to_you() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        let bound = join_channel(&mut eng, 0, p1, ch);
        let passed = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateAndGiveProfile(CreateAndGiveProfile {
                    channel_id: ch,
                    player_id: p1,
                    display: ActorDisplay::Mysterious,
                    visible: true,
                    shared: false,
                    transferrable: true,
                    perm_policy: PermUpdatePolicy::Fixed(FixedPolicy {
                        perms: ChannelPerm::Send | ChannelPerm::View,
                    }),
                }),
            })
            .unwrap()
            .0;
        let ActionResponse::CreateAndGiveProfile(passed) = passed else {
            unreachable!()
        };

        remove_from_channel(&mut eng, 1, p1, ch).unwrap();

        let channel = get_channel(&eng, ch).unwrap();
        assert!(!channel.is_member(p1));
        assert!(channel.get_profile(bound).is_none());
        let left_behind = channel
            .get_profile(passed.profile_id)
            .expect("a name somebody else could wear stays in the channel");
        assert!(left_behind.ownership.owners().is_empty());
    }

    #[test]
    fn removal_exits_the_channel_viewport() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        join_channel(&mut eng, 0, p1, ch);

        let viewport = get_channel(&eng, ch).unwrap().viewport;
        let (_, ctx) = remove_from_channel(&mut eng, 0, p1, ch).unwrap();

        // Losing membership is an exit, not a retraction: p1 keeps everything the channel
        // already told them, they just stop receiving more.
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(&p.cmd, Command::ExitViewport { viewport: v, actor } if *v == viewport && *actor == p1)
        }));
    }

    #[test]
    fn set_loggable_toggles_flag() {
        let mut eng = Engine::new();
        let ch = create_channel(&mut eng, 0, false);

        assert!(!get_channel(&eng, ch).unwrap().loggable);

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::SetLoggable(SetLoggable {
                channel_id: ch,
                loggable: true,
            }),
        })
        .unwrap();

        assert!(get_channel(&eng, ch).unwrap().loggable);
    }

    #[test]
    fn send_message_valid() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        let seat = join_channel(&mut eng, 0, p1, ch);

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, seat, "hello").unwrap();

        assert!(ctx.commands.iter().any(|p| {
            matches!(&p.cmd, Command::AddMessage { channel_id, content, sender_display }
                if *channel_id == ch
                    && content == "hello"
                    && *sender_display == ActorDisplay::Raw(p1))
        }));
    }

    #[test]
    fn send_message_no_send_perm() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        let seat = give_profile(
            &mut eng,
            0,
            p1,
            ch,
            ActorDisplay::Raw(p1),
            ChannelPerm::View.into(),
        );

        assert!(matches!(
            send_message(&mut eng, 0, p1, ch, seat, "hello"),
            Err((ActionError::InsufficientPermissions, _))
        ));
    }

    // Send belongs to the NAME, not to the person. Holding one name that may talk here does not
    // let you talk through another one that may not.
    #[test]
    fn send_asks_the_name_and_not_the_person() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        join_channel(&mut eng, 0, p1, ch);
        let muted = give_profile(
            &mut eng,
            0,
            p1,
            ch,
            ActorDisplay::Mysterious,
            ChannelPerm::View.into(),
        );

        assert!(matches!(
            send_message(&mut eng, 0, p1, ch, muted, "hello"),
            Err((ActionError::InsufficientPermissions, _))
        ));
    }

    #[test]
    fn send_message_profile_not_owned() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let ch = create_channel(&mut eng, 0, false);
        let theirs = join_channel(&mut eng, 0, p2, ch);

        assert!(matches!(
            send_message(&mut eng, 0, p1, ch, theirs, "hello"),
            Err((ActionError::ProfileNotOwned, _))
        ));
    }

    // The host speaks as nobody, holding no name anywhere, and the room is shown System rather
    // than a player. A player has no such option: they are always somebody here.
    #[test]
    fn only_the_host_may_speak_as_nobody() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        join_channel(&mut eng, 0, p1, ch);

        let nameless = |actor| ActionRequest {
            actor,
            timestamp: 0,
            payload: Action::SendMessage(SendMessage {
                channel_id: ch,
                profile_id: None,
                content: "an announcement".into(),
            }),
        };

        assert!(matches!(
            eng.execute(nameless(ActionActor::Player(p1))),
            Err((ActionError::ProfileRequired, _))
        ));

        let (_, ctx) = eng.execute(nameless(ActionActor::System)).unwrap();
        assert!(ctx.commands.iter().any(|p| {
            matches!(&p.cmd, Command::AddMessage { sender_display, .. }
                if *sender_display == ActorDisplay::System)
        }));
    }

    #[test]
    fn joining_does_not_exit_the_channel_viewport() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateAndGiveProfile(CreateAndGiveProfile {
                    channel_id: ch,
                    player_id: p1,
                    display: ActorDisplay::Raw(p1),
                    visible: true,
                    shared: false,
                    transferrable: false,
                    perm_policy: PermUpdatePolicy::Fixed(FixedPolicy {
                        perms: ChannelPerm::Send | ChannelPerm::View,
                    }),
                }),
            })
            .unwrap();

        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::ExitViewport { .. }))
        );
    }

    // A removal must tell the LEAVER, directed. An exit means "no more content is coming", which
    // is indistinguishable from a quiet channel; without a directed update the leaver keeps a
    // channel it believes it can still read and send in. An empty set is how holding nothing here
    // is stated rather than left to be noticed.
    #[test]
    fn removal_tells_the_leaver_they_hold_nothing() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        join_channel(&mut eng, 0, p1, ch);

        let (_, ctx) = remove_from_channel(&mut eng, 0, p1, ch).unwrap();

        // Addressed to the leaver, not to the viewport they just left — anything sent there is by
        // design something they never receive.
        assert!(ctx.commands.iter().any(|p| {
            matches!(
                (&p.cmd, &p.recipient),
                (
                    Command::ProfileAccess { channel_id, profiles },
                    CommandRecipient::Actor(target),
                ) if *channel_id == ch && profiles.is_empty() && *target == p1
            )
        }));
    }

    // ---- groupchat ----

    #[test]
    fn create_groupchat_emits_map_gc() {
        let mut eng = Engine::new();

        let (response, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateGroupchat(CreateGroupchat {}),
            })
            .unwrap();

        let ActionResponse::CreateGroupchat(data) = response else {
            unreachable!()
        };
        let channel_id = get_gc(&eng, data.id).unwrap().channel_id;
        let viewport = get_channel(&eng, channel_id).unwrap().viewport;

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::MapChannel { channel_id: cid, kind: ChannelKind::Groupchat { gc_id, .. } }
                    if *gc_id == data.id && *cid == channel_id)
        }));
    }

    #[test]
    fn add_to_groupchat_system() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let gc = create_gc(&mut eng, 0);

        // without owner flag: member and cache updated, no GcOwnerStatus emitted
        let (_, ctx) = add_to_gc(&mut eng, 0, ActionActor::System, gc, p1, false).unwrap();
        assert!(get_gc(&eng, gc).unwrap().contains_member(p1));
        assert!(get_player(&eng, p1).unwrap().groupchats.contains(&gc));
        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::GcOwnerStatus { .. }))
        );

        // with owner flag: GcOwnerStatus{owner: true} emitted to new owner
        let (_, ctx) = add_to_gc(&mut eng, 0, ActionActor::System, gc, p2, true).unwrap();
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p2)
                && matches!(&p.cmd, Command::GcOwnerStatus { owner: true, gc_id } if *gc_id == gc)
        }));
    }

    #[test]
    fn add_to_groupchat_as_owner_player() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let gc = create_gc(&mut eng, 0);

        add_to_gc(&mut eng, 0, ActionActor::System, gc, p1, true).unwrap();
        add_to_gc(&mut eng, 0, ActionActor::Player(p1), gc, p2, false).unwrap();

        assert!(get_gc(&eng, gc).unwrap().contains_member(p2));
    }

    #[test]
    fn remove_from_groupchat_as_owner_player() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let gc = create_gc(&mut eng, 0);

        add_to_gc(&mut eng, 0, ActionActor::System, gc, p1, true).unwrap();
        add_to_gc(&mut eng, 0, ActionActor::System, gc, p2, false).unwrap();
        remove_from_gc(&mut eng, 0, ActionActor::Player(p1), gc, p2).unwrap();

        assert!(!get_gc(&eng, gc).unwrap().contains_member(p2));
    }

    #[test]
    fn add_to_groupchat_non_owner_player() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let gc = create_gc(&mut eng, 0);

        assert!(add_to_gc(&mut eng, 0, ActionActor::Player(p1), gc, p2, false).is_err());
    }

    #[test]
    fn add_to_groupchat_target_no_contact() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let gc = create_gc(&mut eng, 0);

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(add_to_gc(&mut eng, 0, ActionActor::System, gc, p1, false).is_err());
    }

    #[test]
    fn remove_from_groupchat_not_member() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let gc = create_gc(&mut eng, 0);

        assert!(remove_from_gc(&mut eng, 0, ActionActor::System, gc, p1).is_err());
    }

    #[test]
    fn set_gc_owner_emits_status_cmds() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let gc = create_gc(&mut eng, 0);

        add_to_gc(&mut eng, 0, ActionActor::System, gc, p1, true).unwrap();
        add_to_gc(&mut eng, 0, ActionActor::System, gc, p2, false).unwrap();

        let (_, ctx) = set_gc_owner(&mut eng, 0, ActionActor::System, gc, Some(p2)).unwrap();

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(&p.cmd, Command::GcOwnerStatus { owner: false, gc_id } if *gc_id == gc)
        }));
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p2)
                && matches!(&p.cmd, Command::GcOwnerStatus { owner: true, gc_id } if *gc_id == gc)
        }));
    }

    // ---- lounge ----

    #[test]
    fn create_basic_lounge_participants_in_channel() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let (_, ch) = create_lounge(
            &mut eng,
            0,
            LoungeVariant::Basic {
                contactor_id: p1,
                contacted_id: p2,
            },
        );

        let channel = get_channel(&eng, ch).unwrap();
        assert!(channel.get_member(p1).is_some());
        assert!(channel.get_member(p2).is_some());
    }

    #[test]
    fn create_lounge_updates_player_caches() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let (lounge_id, _) = create_lounge(
            &mut eng,
            0,
            LoungeVariant::Basic {
                contactor_id: p1,
                contacted_id: p2,
            },
        );

        assert!(get_player(&eng, p1).unwrap().lounges.contains(&lounge_id));
        assert!(get_player(&eng, p2).unwrap().lounges.contains(&lounge_id));
    }

    #[test]
    fn create_lounge_emits_map_lounge() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let (response, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateLounge(CreateLounge {
                    variant: LoungeVariant::Basic {
                        contactor_id: p1,
                        contacted_id: p2,
                    },
                }),
            })
            .unwrap();

        let ActionResponse::CreateLounge(data) = response else {
            unreachable!()
        };

        let viewport = get_channel(&eng, data.channel_id).unwrap().viewport;

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::MapChannel { channel_id, kind: ChannelKind::Lounge { lounge_id, .. } }
                    if *lounge_id == data.lounge_id && *channel_id == data.channel_id)
        }));
    }

    #[test]
    fn leave_lounge_removes_from_channel() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let (lounge_id, ch) = create_lounge(
            &mut eng,
            0,
            LoungeVariant::Basic {
                contactor_id: p1,
                contacted_id: p2,
            },
        );

        leave_lounge(&mut eng, 0, p1, lounge_id).unwrap();

        assert!(get_channel(&eng, ch).unwrap().get_member(p1).is_none());
    }

    #[test]
    fn leave_lounge_updates_player_cache() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let (lounge_id, _) = create_lounge(
            &mut eng,
            0,
            LoungeVariant::Basic {
                contactor_id: p1,
                contacted_id: p2,
            },
        );

        leave_lounge(&mut eng, 0, p1, lounge_id).unwrap();

        assert!(!get_player(&eng, p1).unwrap().lounges.contains(&lounge_id));
    }

    // ---- bug ----

    // The viewport the bug's relay rides. Every visibility test below creates exactly one bug, and
    // this is what an assertion about access to it has to name: EnterViewport carries the key
    // alone, the kind being a property of the viewport rather than of the grant.
    fn only_bug_viewport(eng: &Engine) -> ViewportKey {
        eng.world
            .bugs
            .values()
            .next()
            .expect("the test should have created a bug")
            .viewport
    }

    #[test]
    fn create_bug_stored_in_world() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(data) = response else {
            unreachable!()
        };

        assert!(get_bug(&eng, data.id).is_ok());
    }

    #[test]
    fn create_bug_registered_in_player_bugs() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(data) = response else {
            unreachable!()
        };

        assert!(get_player(&eng, p1).unwrap().bugs.contains(&data.id));
    }

    #[test]
    fn create_bug_emits_new_bug_command() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(data) = response else {
            unreachable!()
        };

        let viewport = eng.world.get_bug(data.id).unwrap().viewport;

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::NewBug { bug_key } if *bug_key == data.id)
        }));
    }

    #[test]
    fn create_bug_notifies_target_with_context() {
        use lawliet_types::bug::BugContext;

        // Custody bug -> target is notified with the Custody context.
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(
                    &p.cmd,
                    Command::Bugged {
                        context: BugContext::Custody
                    }
                )
        }));

        // Ability bug -> target is notified with the Explicit context, and the owner-identifying
        // ability key is stripped (the target learns *that* they're bugged, never *who* by).
        let owner = add_player(&mut eng, 0, Role::Civilian, "owner");
        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                actor_id: owner,
                ability_name: AbilityName::Gun,
                variant: 0,
                transferrable: false,
                volatile: false,
            },
        );
        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Ability(ab),
                }),
            })
            .unwrap();
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(
                    &p.cmd,
                    Command::Bugged {
                        context: BugContext::Explicit
                    }
                )
        }));
    }

    #[test]
    fn create_bug_invalid_target_fails() {
        let mut eng = Engine::new();

        assert!(
            eng.execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: ActorKey::default(),
                    source: BugSource::Custody,
                }),
            })
            .is_err()
        );
    }

    #[test]
    fn create_bug_invalid_ability_source_fails() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        assert!(
            eng.execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Ability(AbilityKey::default()),
                }),
            })
            .is_err()
        );
    }

    #[test]
    fn archive_bug_disables_bug() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(data) = response else {
            unreachable!()
        };

        assert!(get_bug(&eng, data.id).unwrap().enabled);

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::ArchiveBug(ArchiveBug { bug_id: data.id }),
        })
        .unwrap();

        assert!(!get_bug(&eng, data.id).unwrap().enabled);
    }

    #[test]
    fn archive_bug_emits_archive_command() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(create_data) = response else {
            unreachable!()
        };

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::ArchiveBug(ArchiveBug {
                    bug_id: create_data.id,
                }),
            })
            .unwrap();

        // Archiving leaves the bug (and its viewport) in place — it only stops the relay — so
        // the notice still reaches everyone who could read it.
        let viewport = eng.world.get_bug(create_data.id).unwrap().viewport;

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::ArchiveBug { bug_key } if *bug_key == create_data.id)
        }));
    }

    #[test]
    fn archive_bug_invalid_id_fails() {
        let mut eng = Engine::new();

        assert!(
            eng.execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::ArchiveBug(ArchiveBug {
                    bug_id: BugKey::default(),
                }),
            })
            .is_err()
        );
    }

    #[test]
    fn archive_bug_stays_in_world() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(data) = response else {
            unreachable!()
        };

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::ArchiveBug(ArchiveBug { bug_id: data.id }),
        })
        .unwrap();

        assert!(get_bug(&eng, data.id).is_ok());
    }

    #[test]
    fn destroy_bug_removed_from_world() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(data) = response else {
            unreachable!()
        };

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::DestroyBug(DestroyBug { bug_id: data.id }),
        })
        .unwrap();

        assert!(get_bug(&eng, data.id).is_err());
    }

    #[test]
    fn destroy_bug_removed_from_player_bugs() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(data) = response else {
            unreachable!()
        };

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::DestroyBug(DestroyBug { bug_id: data.id }),
        })
        .unwrap();

        assert!(!get_player(&eng, p1).unwrap().bugs.contains(&data.id));
    }

    #[test]
    fn destroy_bug_emits_archive_command() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(create_data) = response else {
            unreachable!()
        };

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::DestroyBug(DestroyBug {
                    bug_id: create_data.id,
                }),
            })
            .unwrap();

        // Destroying a bug archives it rather than deleting it, and the notice is addressed to
        // the bug's own viewport so it reaches whoever was reading the relay.
        assert!(ctx.commands.iter().any(|p| {
            matches!(p.recipient, CommandRecipient::Viewport(_))
                && matches!(&p.cmd, Command::ArchiveBug { bug_key } if *bug_key == create_data.id)
        }));
    }

    #[test]
    fn destroy_bug_invalid_id_fails() {
        let mut eng = Engine::new();

        assert!(
            eng.execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::DestroyBug(DestroyBug {
                    bug_id: BugKey::default(),
                }),
            })
            .is_err()
        );
    }

    #[test]
    fn send_message_relays_to_enabled_bug_on_loggable_channel() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, true);
        let seat = join_channel(&mut eng, 0, p1, ch);

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(bug_data) = response else {
            unreachable!()
        };

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, seat, "hello").unwrap();

        let viewport = eng.world.get_bug(bug_data.id).unwrap().viewport;

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(viewport)
                && matches!(&p.cmd, Command::AddBugMessage { bug_key, .. } if *bug_key == bug_data.id)
        }));
    }

    #[test]
    fn send_message_no_relay_on_non_loggable_channel() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        let seat = join_channel(&mut eng, 0, p1, ch);

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::CreateBug(CreateBug {
                target_id: p1,
                source: BugSource::Custody,
            }),
        })
        .unwrap();

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, seat, "hello").unwrap();

        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::AddBugMessage { .. }))
        );
    }

    #[test]
    fn send_message_no_relay_for_archived_bug() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, true);
        let seat = join_channel(&mut eng, 0, p1, ch);

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(bug_data) = response else {
            unreachable!()
        };

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::ArchiveBug(ArchiveBug {
                bug_id: bug_data.id,
            }),
        })
        .unwrap();

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, seat, "hello").unwrap();

        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::AddBugMessage { .. }))
        );
    }

    #[test]
    fn send_message_relay_correct_content_and_display() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, true);
        let seat = join_channel(&mut eng, 0, p1, ch);

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: p1,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(bug_data) = response else {
            unreachable!()
        };

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, seat, "secret message").unwrap();

        assert!(ctx.commands.iter().any(|p| {
            matches!(&p.cmd, Command::AddBugMessage { bug_key, display, content }
                if *bug_key == bug_data.id
                    && *display == ActorDisplay::Raw(p1)
                    && content == "secret message")
        }));
    }

    #[test]
    fn visibility_ability_bug_visible_to_owner() {
        let mut eng = Engine::new();
        let owner = add_player(&mut eng, 0, Role::Civilian, "owner");
        let target = add_player(&mut eng, 0, Role::Civilian, "target");
        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                actor_id: owner,
                ability_name: AbilityName::Gun,
                variant: 0,
                transferrable: false,
                volatile: false,
            },
        );

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: target,
                    source: BugSource::Ability(ab),
                }),
            })
            .unwrap();

        let bug_viewport = only_bug_viewport(&eng);
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(owner)
                && matches!(&p.cmd, Command::EnterViewport { viewport, actor }
                    if *viewport == bug_viewport && *actor == owner)
        }));
    }

    #[test]
    fn visibility_ability_bug_no_owner_no_access() {
        let mut eng = Engine::new();
        let target = add_player(&mut eng, 0, Role::Civilian, "target");

        let (response, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::AddAbility(AddAbility {
                    ability_name: AbilityName::Gun,
                    variant: 0,
                    transferrable: false,
                }),
            })
            .unwrap();
        let ActionResponse::AddAbility(ab_data) = response else {
            unreachable!()
        };

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: target,
                    source: BugSource::Ability(ab_data.id),
                }),
            })
            .unwrap();

        let bug_viewport = only_bug_viewport(&eng);
        assert!(!ctx.commands.iter().any(
            |p| matches!(&p.cmd, Command::EnterViewport { viewport, .. } if *viewport == bug_viewport)
        ));
    }

    #[test]
    fn visibility_ability_bug_owner_nopresence_not_visible() {
        let mut eng = Engine::new();
        let owner = add_player(&mut eng, 0, Role::Civilian, "owner");
        let target = add_player(&mut eng, 0, Role::Civilian, "target");
        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                actor_id: owner,
                ability_name: AbilityName::Gun,
                variant: 0,
                transferrable: false,
                volatile: false,
            },
        );

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::CreateBug(CreateBug {
                target_id: target,
                source: BugSource::Ability(ab),
            }),
        })
        .unwrap();

        // Incarcerated gives NoPresence — visibility update is triggered inside AddState
        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::AddState(AddState {
                    actor_id: owner,
                    state: State::Incarcerated,
                }),
            })
            .unwrap();

        let bug_viewport = only_bug_viewport(&eng);
        assert!(!ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(owner)
                && matches!(&p.cmd, Command::EnterViewport { viewport, actor }
                    if *viewport == bug_viewport && *actor == owner)
        }));
    }

    #[test]
    fn visibility_custody_bug_visible_to_receiver() {
        let mut eng = Engine::new();
        let receiver = add_player(&mut eng, 0, Role::Civilian, "receiver");
        let target = add_player(&mut eng, 0, Role::Civilian, "target");
        quick_passive(
            &mut eng,
            0,
            receiver,
            PassiveType::CustodyBugReceiver,
            false,
        );

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: target,
                    source: BugSource::Custody,
                }),
            })
            .unwrap();

        let bug_viewport = only_bug_viewport(&eng);
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(receiver)
                && matches!(&p.cmd, Command::EnterViewport { viewport, actor }
                    if *viewport == bug_viewport && *actor == receiver)
        }));
    }

    #[test]
    fn new_bug_is_addressed_to_its_viewport_before_access_is_granted() {
        let mut eng = Engine::new();
        let owner = add_player(&mut eng, 0, Role::Civilian, "owner");
        let target = add_player(&mut eng, 0, Role::Civilian, "target");
        let ab = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                actor_id: owner,
                ability_name: AbilityName::Gun,
                variant: 0,
                transferrable: false,
                volatile: false,
            },
        );

        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::CreateBug(CreateBug {
                    target_id: target,
                    source: BugSource::Ability(ab),
                }),
            })
            .unwrap();

        // Everything the bug's viewport says about itself must land there before anyone is
        // admitted. That ordering is what makes a viewport legible: whoever enters learns what the
        // viewport IS from the first things in it — first that it is a bug's, then which bug.
        let bug_viewport = only_bug_viewport(&eng);
        let position = |find: &dyn Fn(&Command) -> bool| {
            ctx.commands.iter().position(|p| {
                p.recipient == CommandRecipient::Viewport(bug_viewport) && find(&p.cmd)
            })
        };

        let map_pos = position(&|cmd| matches!(cmd, Command::MapViewport { .. })).unwrap();
        let new_bug_pos = position(&|cmd| matches!(cmd, Command::NewBug { .. })).unwrap();
        let enter_pos = ctx
            .commands
            .iter()
            .position(
                |p| matches!(&p.cmd, Command::EnterViewport { viewport, .. } if *viewport == bug_viewport),
            )
            .unwrap();

        assert!(map_pos < new_bug_pos);
        assert!(new_bug_pos < enter_pos);
    }

    // ---- update_contact_channels ----

    #[test]
    fn no_contact_clears_lounge_perms() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let (_, ch) = create_lounge(
            &mut eng,
            0,
            LoungeVariant::Basic {
                contactor_id: p1,
                contacted_id: p2,
            },
        );

        add_state(&mut eng, 0, p1, State::Dead);

        assert!(perms(&eng, ch, p1).is_empty());
    }

    #[test]
    fn no_contact_cleared_restores_lounge_perms() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");

        let (_, ch) = create_lounge(
            &mut eng,
            0,
            LoungeVariant::Basic {
                contactor_id: p1,
                contacted_id: p2,
            },
        );

        add_state(&mut eng, 0, p1, State::Dead);
        remove_state(&mut eng, 0, p1, State::Dead);

        let restored = perms(&eng, ch, p1);
        assert!(restored.contains(ChannelPerm::Send));
        assert!(restored.contains(ChannelPerm::View));
    }
}
