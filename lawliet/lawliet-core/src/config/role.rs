use indexmap::IndexMap;

pub use lawliet_types::role::Role;

use crate::{
    actor::ActorLinkType,
    channel::ProfileBlueprint,
    config::{
        ability::{AbilityIdentifier, AbilityName},
        world::WorldChannelName,
    },
    passive::{ContactLogType, PassiveType},
};

pub use lawliet_types::channel::{BlueprintDisplayKind, ContactPolicy, PermUpdatePolicy};

// A seat in a world channel that comes with the role.
//
// This is how a channel nobody belongs to by default gets its guest list: L and Watari's line has
// no blueprint of its own, and is reached by being one of them. The blueprint says what the seat
// is worth once you have it, which for an ordinary line between two people is simply contact.
//
// Given and taken away by GiveRole, so a role change moves you out of the old role's rooms and
// into the new one's.
#[derive(PartialEq, Eq, Clone)]
pub struct RoleWorldChannelProfile {
    pub channel_name: WorldChannelName,
    pub blueprint: ProfileBlueprint,
}

// TODO:
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
    pub world_channel_profiles: Vec<RoleWorldChannelProfile>,
}

pub type RoleConfigMap = IndexMap<Role, RoleConfig>;

// The seat L and Watari share, which is the only thing that gets anybody into that channel. An
// ordinary two-way line once you are in it: talk and listen unless you have been cut off.
fn l_and_watari_line() -> RoleWorldChannelProfile {
    RoleWorldChannelProfile {
        channel_name: WorldChannelName::LAndWatari,
        blueprint: ProfileBlueprint {
            start_visible: true,
            display_kind: BlueprintDisplayKind::OwnerRaw,
            perm_policy: PermUpdatePolicy::Contact(ContactPolicy {}),
        },
    }
}

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
                // RoleAbility {
                //     identifier: AbilityIdentifier {
                //         name: AbilityName::ShinigamiEyeDeal,
                //         variant: 0,
                //     },
                //     transferrable: false,
                // },
            ],
            passives: vec![],
            notebooks: vec![RoleNotebook { fake: false }],
            actor_links: vec![],
            world_channel_profiles: vec![],
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
            world_channel_profiles: vec![],
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
            world_channel_profiles: vec![l_and_watari_line()],
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
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Prosecute,
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
            world_channel_profiles: vec![l_and_watari_line()],
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
            world_channel_profiles: vec![],
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
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Prosecute,
                        variant: 0,
                    },
                    transferrable: false,
                },
                // One charge that never resets: a single reroll for the whole game.
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::TrueNameReroll,
                        variant: 0,
                    },
                    transferrable: false,
                },
                RoleAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Ipp,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_profiles: vec![],
        },
    );

    map.insert(
        Role::Civilian,
        RoleConfig {
            abilities: vec![],
            passives: vec![],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_profiles: vec![],
        },
    );

    map.insert(
        Role::RogueCivilian,
        RoleConfig {
            abilities: vec![],
            passives: vec![],
            notebooks: vec![RoleNotebook { fake: false }],
            actor_links: vec![],
            world_channel_profiles: vec![],
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
            world_channel_profiles: vec![],
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
            world_channel_profiles: vec![],
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
            world_channel_profiles: vec![],
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
            world_channel_profiles: vec![],
        },
    );

    map.insert(
        Role::Mello,
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
                        name: AbilityName::AnonymousKidnap,
                        variant: 0,
                    },
                    transferrable: false,
                },
            ],
            passives: vec![RolePassive {
                passive_type: PassiveType::ContactLogs(ContactLogType::Odd),
                transferrable: true,
            }],
            notebooks: vec![],
            actor_links: vec![],
            world_channel_profiles: vec![],
        },
    );

    map
}
