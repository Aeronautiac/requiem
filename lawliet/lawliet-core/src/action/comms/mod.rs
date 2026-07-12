pub mod bug;
pub mod channel;
pub mod groupchat;
pub mod lounge;
pub mod update_contact_channels;

#[cfg(test)]
mod comms_tests {
    use indexmap::indexset;
    use lawliet_types::command::CommandRecipient;

    use crate::{
        action::{
            Action, ActionActor, ActionRequest, ActionResponse,
            ability::{add_ability::AddAbility, create_and_give_ability::CreateAndGiveAbility},
            actor::add_state::AddState,
            comms::{
                bug::{archive_bug::ArchiveBug, create_bug::CreateBug, destroy_bug::DestroyBug},
                channel::set_loggable::SetLoggable,
                groupchat::create_groupchat::CreateGroupchat,
                lounge::create_lounge::CreateLounge,
            },
        },
        actor::{ActorDisplay, state::State},
        bug::BugSource,
        channel::{ChannelMember, ChannelPermission},
        command::Command,
        common::{AbilityKey, ActorKey, BugKey},
        config::{ability::AbilityName, role::Role},
        engine::Engine,
        helpers::{get_bug, get_channel, get_gc, get_player},
        lounge::LoungeVariant,
        passive::PassiveType,
        test_helpers::*,
    };

    // ---- channel ----

    #[test]
    fn set_member_adds_player() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        assert!(get_channel(&eng, ch).unwrap().get_member(p1).is_some());
    }

    #[test]
    fn set_member_removes_player() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();
        set_member(&mut eng, 0, p1, ch, None).unwrap();

        assert!(get_channel(&eng, ch).unwrap().get_member(p1).is_none());
    }

    #[test]
    fn set_member_emits_update_channel_view() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        let (_, ctx) = set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(&p.cmd, Command::UpdateChannelView { channel_id, .. } if *channel_id == ch)
        }));
    }

    #[test]
    fn set_member_removal_emits_remove_channel() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        let (_, ctx) = set_member(&mut eng, 0, p1, ch, None).unwrap();

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(p1)
                && matches!(&p.cmd, Command::RemoveChannel { channel_id } if *channel_id == ch)
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

        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p1), "hello").unwrap();

        assert!(ctx.commands.iter().any(|p| {
            matches!(&p.cmd, Command::AddMessage { channel_id, content, sender_display }
                if *channel_id == ch
                    && content == "hello"
                    && *sender_display == ActorDisplay::Raw(p1))
        }));
    }

    #[test]
    fn send_message_not_a_member() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        assert!(send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p1), "hello").is_err());
    }

    #[test]
    fn send_message_no_send_perm() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::View.into(),
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        assert!(send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p1), "hello").is_err());
    }

    #[test]
    fn send_message_display_not_owned() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let ch = create_channel(&mut eng, 0, false);

        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        // p1 tries to send as p2 which they do not own
        assert!(send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p2), "hello").is_err());
    }

    #[test]
    fn set_member_add_does_not_emit_remove_channel() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        let (_, ctx) = set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::RemoveChannel { .. }))
        );
    }

    #[test]
    fn set_member_remove_does_not_emit_update_channel_view() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);

        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        let (_, ctx) = set_member(&mut eng, 0, p1, ch, None).unwrap();

        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::UpdateChannelView { .. }))
        );
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

        assert!(ctx.commands.iter().any(|p| {
            p.recipient.is_system()
                && matches!(&p.cmd, Command::MapGc { gc_id, channel_id: cid }
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

        assert!(ctx.commands.iter().any(|p| {
            p.recipient.is_system()
                && matches!(&p.cmd, Command::MapLounge { lounge_id, channel_id }
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

        assert!(ctx.commands.iter().any(|p| {
            p.recipient.is_system()
                && matches!(&p.cmd, Command::NewBug { bug_key } if *bug_key == data.id)
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

        assert!(ctx.commands.iter().any(|p| {
            p.recipient.is_system()
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
    fn destroy_bug_emits_delete_command() {
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

        assert!(ctx.commands.iter().any(|p| {
            p.recipient.is_system()
                && matches!(&p.cmd, Command::DeleteBug { bug_id } if *bug_id == create_data.id)
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
        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

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

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p1), "hello").unwrap();

        assert!(ctx.commands.iter().any(|p| {
            p.recipient.is_system()
                && matches!(&p.cmd, Command::AddBugMessage { bug_key, .. } if *bug_key == bug_data.id)
        }));
    }

    #[test]
    fn send_message_no_relay_on_non_loggable_channel() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let ch = create_channel(&mut eng, 0, false);
        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 0,
            payload: Action::CreateBug(CreateBug {
                target_id: p1,
                source: BugSource::Custody,
            }),
        })
        .unwrap();

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p1), "hello").unwrap();

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
        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

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

        let (_, ctx) = send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p1), "hello").unwrap();

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
        set_member(
            &mut eng,
            0,
            p1,
            ch,
            Some(ChannelMember {
                perms: ChannelPermission::Send | ChannelPermission::View,
                displays: indexset![ActorDisplay::Raw(p1)],
            }),
        )
        .unwrap();

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

        let (_, ctx) =
            send_message(&mut eng, 0, p1, ch, ActorDisplay::Raw(p1), "secret message").unwrap();

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

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(owner)
                && matches!(&p.cmd, Command::SetBugVisibility { visible: true, .. })
        }));
    }

    #[test]
    fn visibility_ability_bug_no_owner_no_set_visibility() {
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

        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::SetBugVisibility { visible: true, .. }))
        );
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

        assert!(!ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(owner)
                && matches!(&p.cmd, Command::SetBugVisibility { visible: true, .. })
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

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(receiver)
                && matches!(&p.cmd, Command::SetBugVisibility { visible: true, .. })
        }));
    }

    #[test]
    fn visibility_clears_before_setting() {
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

        let clear_pos = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::ClearBugVisibily { .. }))
            .unwrap();
        let set_pos = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::SetBugVisibility { visible: true, .. }))
            .unwrap();

        assert!(clear_pos < set_pos);
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

        assert!(
            get_channel(&eng, ch)
                .unwrap()
                .get_member(p1)
                .unwrap()
                .perms
                .is_empty()
        );
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

        let member = get_channel(&eng, ch)
            .unwrap()
            .get_member(p1)
            .unwrap()
            .clone();
        assert!(member.perms.contains(ChannelPermission::Send));
        assert!(member.perms.contains(ChannelPermission::View));
    }
}
