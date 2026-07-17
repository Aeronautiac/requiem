pub mod add_to_world_channels;
pub mod create_orgs;
pub mod initialize_engine;
pub mod initialize_world;
pub mod next_iteration;
pub mod set_random_seed;
pub mod set_world_channel_override;
pub mod update_world_channel_perms;

#[cfg(test)]
mod world_tests {
    use crate::{
        actor::{
            player::{OverrideResolver, OverrideSource, WorldChannelOverride},
            state::State,
        },
        channel::{ChannelPermission, ChannelPermissions},
        config::{role::Role, world::WorldChannelName},
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
