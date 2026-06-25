use indexmap::IndexMap;

pub use lawliet_types::role::Role;

use crate::{
    actor::{ActorLinkType, player::WorldChannelOverride},
    channel::{ChannelPermission, ChannelPermissions},
    config::{
        ability::{AbilityIdentifier, AbilityName},
        world::WorldChannelName,
    },
    passive::{ContactLogType, PassiveType},
};

#[derive(PartialEq, Eq, Clone)]
pub struct RoleWorldChannelOverride {
    pub channel_name: WorldChannelName,
    pub override_data: WorldChannelOverride,
}

// TODO:
// - Add organization configurations. Certain roles spawn in organizations with certain permissions.
// - Possibly change roles being hardcoded enums and instead make them strings or identifiers. This
// would allow dynamic role creation on the host's end and wouldn't require much refactoring because
// the engine doesn't hardcode anything.

#[derive(PartialEq, Eq, Clone)]
pub struct RolePassive {
    pub passive_type: PassiveType,
    pub transferrable: bool,
}

#[derive(PartialEq, Eq, Clone)]
pub struct RoleAbility {
    pub identifier: AbilityIdentifier,
    pub transferrable: bool,
}

#[derive(PartialEq, Eq, Clone)]
pub struct RoleNotebook {
    pub fake: bool,
}

#[derive(PartialEq, Eq, Clone)]
pub struct RoleLink {
    pub role: Role,
    pub link_type: ActorLinkType,
}

#[derive(PartialEq, Eq, Clone)]
pub struct RoleConfig {
    pub abilities: Vec<RoleAbility>,
    pub passives: Vec<RolePassive>,
    pub notebooks: Vec<RoleNotebook>,
    pub actor_links: Vec<RoleLink>,
    pub world_channel_overrides: Vec<RoleWorldChannelOverride>,
}

pub type RoleConfigMap = IndexMap<Role, RoleConfig>;

pub fn default_role_config() -> RoleConfigMap {
    let mut map = RoleConfigMap::new();

    map.insert(
        Role::Kira,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::UnderTheRadar,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousAnnouncement,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![],
            notebooks: vec![RoleNotebook { fake: false }],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::SecondKira,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousAnnouncement,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::UnderTheRadar,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::KiraConnection,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::TrueNameReveal,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::NotebookReveal,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![RolePassive {
                passive_type: PassiveType::OwnedNotebookBlock,
                transferrable: false,
            }],
            notebooks: vec![RoleNotebook { fake: false }],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::L,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousAnnouncement,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousProsecute,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![RolePassive {
                passive_type: PassiveType::CustodyBugReceiver,
                transferrable: false,
            }],
            notebooks: vec![],
            actor_links: vec![
                RoleLink {
                    role: Role::Watari,
                    link_type: ActorLinkType::Life,
                },
                RoleLink {
                    role: Role::Watari,
                    link_type: ActorLinkType::Passive,
                },
            ],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::Watari,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Bug,
                        variant: 0,
                    },
                    transferrable: true,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousContact,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![
                RolePassive {
                    passive_type: PassiveType::ContactLogs(ContactLogType::Full),
                    transferrable: true,
                },
                RolePassive {
                    passive_type: PassiveType::CustodyBugReceiver,
                    transferrable: false,
                },
            ],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::BeyondBirthday,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Pseudocide,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::TrueNameReveal,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::NotebookReveal,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![RolePassive {
                passive_type: PassiveType::VolatileEyes,
                transferrable: false,
            }],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::PrivateInvestigator,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Autopsy,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousContact,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::NewsAnchor,
        RoleConfig {
            abilities: vec![RoleAbility {
                identifier: AbilityIdentifier {
                    name: AbilityName::CivilianArrest,
                    variant: 0,
                },
                transferrable: false,
            }],
            passives: vec![RolePassive {
                passive_type: PassiveType::VoteAmplification { multiplier: 2 },
                transferrable: false,
            }],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![RoleWorldChannelOverride {
                channel_name: WorldChannelName::News,
                override_data: WorldChannelOverride {
                    default_perms: ChannelPermission::Send | ChannelPermission::View,
                    force_perms: ChannelPermissions::EMPTY,
                },
            }],
        },
    );

    map.insert(
        Role::Civilian,
        RoleConfig {
            abilities: vec![],
            passives: vec![],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::RogueCivilian,
        RoleConfig {
            abilities: vec![],
            passives: vec![],
            notebooks: vec![RoleNotebook { fake: false }],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::Poser,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::FalseAnonymousContact,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousAnnouncement,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::ConArtist,
        RoleConfig {
            abilities: vec![RoleAbility {
                identifier: AbilityIdentifier {
                    name: AbilityName::FabricateLounge,
                    variant: 0,
                },
                transferrable: false,
            }],
            passives: vec![],
            notebooks: vec![RoleNotebook { fake: true }],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::WantedCivilian,
        RoleConfig {
            abilities: vec![
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Bug,
                        variant: 0,
                    },
                    transferrable: true,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::TapIn,
                        variant: 1,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![RolePassive {
                passive_type: PassiveType::Wanted,
                transferrable: false,
            }],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::Near,
        RoleConfig {
            abilities: vec![RoleAbility {
                identifier: AbilityIdentifier {
                    name: AbilityName::AnonymousAnnouncement,
                    variant: 0,
                },
                transferrable: false,
            }],
            passives: vec![RolePassive {
                passive_type: PassiveType::ContactLogs(ContactLogType::Even),
                transferrable: true,
            }],
            notebooks: vec![RoleNotebook { fake: true }],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map.insert(
        Role::Mello,
        RoleConfig {
            abilities: vec![RoleAbility {
                identifier: AbilityIdentifier {
                    name: AbilityName::AnonymousAnnouncement,
                    variant: 0,
                },
                transferrable: false,
            }],
            passives: vec![RolePassive {
                passive_type: PassiveType::ContactLogs(ContactLogType::Odd),
                transferrable: true,
            }],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_overrides: vec![],
        },
    );

    map
}
