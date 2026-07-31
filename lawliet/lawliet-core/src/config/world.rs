use indexmap::IndexMap;

use crate::{channel::ProfileBlueprint, chargepool::PoolSpecifier};

pub use lawliet_types::channel::{
    BlueprintDisplayKind, ChannelPerm, NewsPolicy, PermUpdatePolicy, PresencePolicy,
};
pub use lawliet_types::organization::OrganizationName;
pub use lawliet_types::world::{WorldChannelName, WorldChargePoolName};

pub struct WorldConfig {
    pub charge_pools: IndexMap<WorldChargePoolName, PoolSpecifier>,
    // What each world channel gives every player, if it gives them anything at all.
    //
    // A blueprint is the whole of a world channel's configuration: it says everyone belongs here
    // and names the rule deciding what that is worth. None is a channel you are put into by
    // something happening to you rather than by existing — the prison — and whatever does the
    // putting owns its membership.
    pub world_channels: IndexMap<WorldChannelName, Option<ProfileBlueprint>>,
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

        // Everyone listens; the anchor talks; a blackout takes the whole thing off the air.
        channels.insert(
            WorldChannelName::News,
            Some(ProfileBlueprint {
                start_visible: true,
                display_kind: BlueprintDisplayKind::OwnerRaw,
                perm_policy: PermUpdatePolicy::News(NewsPolicy {}),
            }),
        );

        // The town square: talk and listen for as long as you are present to.
        channels.insert(
            WorldChannelName::General,
            Some(ProfileBlueprint {
                start_visible: true,
                display_kind: BlueprintDisplayKind::OwnerRaw,
                perm_policy: PermUpdatePolicy::Presence(PresencePolicy {
                    perms: ChannelPerm::Send | ChannelPerm::View,
                }),
            }),
        );

        // Neither of these is a channel you are in for existing, so neither hands out a seat.
        //
        // The prison is one you are put into, and the incarceration owns who is in it. L and
        // Watari's line is one you are in because of what you are, and role config hands it out —
        // which is why it needs no rule of its own here: it is an ordinary contact channel with an
        // unusual guest list.
        channels.insert(WorldChannelName::Prison, None);
        channels.insert(WorldChannelName::LAndWatari, None);

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
