use indexmap::IndexMap;

use crate::{
    actor::modifier::{Modifier, Modifiers},
    channel::{ChannelPermission, ChannelPermissions},
    chargepool::PoolSpecifier,
};

pub use lawliet_types::organization::OrganizationName;
pub use lawliet_types::world::{WorldChannelName, WorldChargePoolName};

pub struct WorldChannelConfig {
    pub default_perms: ChannelPermissions,
    pub send_blocking: Modifiers,
    pub view_blocking: Modifiers,
}

pub struct WorldConfig {
    pub charge_pools: IndexMap<WorldChargePoolName, PoolSpecifier>,
    pub world_channels: IndexMap<WorldChannelName, WorldChannelConfig>,
    // Organizations spawned once on world initialization (see CreateOrgs).
    pub default_orgs: Vec<OrganizationName>,
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
                view_blocking: Modifier::NoPresence.into(),
            },
        );
        channels.insert(
            WorldChannelName::Prison,
            WorldChannelConfig {
                default_perms: ChannelPermissions::EMPTY,
                send_blocking: Modifier::AbsoluteNoContact.into(),
                view_blocking: Modifier::AbsoluteNoContact.into(),
            },
        );
        channels.insert(
            WorldChannelName::LAndWatari,
            WorldChannelConfig {
                default_perms: ChannelPermissions::EMPTY,
                send_blocking: Modifier::NoContact.into(),
                view_blocking: Modifier::NoPresence.into(),
            },
        );

        WorldConfig {
            charge_pools: pools,
            world_channels: channels,
            default_orgs: vec![
                OrganizationName::KK,
                OrganizationName::TF,
                OrganizationName::SPK,
            ],
        }
    }
}
