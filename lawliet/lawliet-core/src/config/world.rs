use indexmap::IndexMap;

use crate::{
    actor::modifier::{Modifier, Modifiers},
    channel::{ChannelPermission, ChannelPermissions},
    chargepool::PoolSpecifier,
};

pub use lawliet_types::world::{WorldChargePoolName, WorldChannelName};

pub struct WorldChannelConfig {
    pub default_perms: ChannelPermissions,
    pub send_blocking: Modifiers,
    pub view_blocking: Modifiers,
}

pub struct WorldConfig {
    pub charge_pools: IndexMap<WorldChargePoolName, PoolSpecifier>,
    pub world_channels: IndexMap<WorldChannelName, WorldChannelConfig>,
}

impl WorldConfig {
    pub fn new() -> Self {
        let mut pools = IndexMap::new();
        pools.insert(
            WorldChargePoolName::Prosecution,
            PoolSpecifier {
                charges: 2,
                reset_time: 1,
            },
        );

        let mut channels = IndexMap::new();
        channels.insert(
            WorldChannelName::News,
            WorldChannelConfig {
                default_perms: ChannelPermission::View.into(),
                send_blocking: Modifier::NoContact.into(),
                view_blocking: Modifier::NoPresence.into(),
            },
        );
        channels.insert(
            WorldChannelName::General,
            WorldChannelConfig {
                default_perms: ChannelPermission::Send | ChannelPermission::View,
                send_blocking: Modifier::NoContact.into(),
                view_blocking: Modifier::NoContact.into(),
            },
        );
        channels.insert(
            WorldChannelName::Prison,
            WorldChannelConfig {
                default_perms: ChannelPermissions::EMPTY,
                send_blocking: Modifiers::EMPTY,
                view_blocking: Modifiers::EMPTY,
            },
        );

        WorldConfig {
            charge_pools: pools,
            world_channels: channels,
        }
    }
}
