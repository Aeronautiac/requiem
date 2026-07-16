use indexmap::IndexMap;

pub use lawliet_types::ability::AbilityName;

use crate::{
    chargepool::{ChargeCondition, ChargeConditions, PoolLinkType, PoolSpecifier},
    common::{IterationCount, LinkWeight, Variant},
    config::{actor::ActorChargePoolName, world::WorldChargePoolName},
};

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct AbilityIdentifier {
    pub name: AbilityName,
    pub variant: Variant,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum ConfigPoolLinkDetails {
    Individual(PoolSpecifier),  // the ability creates its own charge pool
    Actor(ActorChargePoolName), // actors and the world have a map of pool names to charge pools
    World(WorldChargePoolName),
}

// no Ord: ChargeConditions (BitFlags) isn't ordered
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ConfigPoolLink {
    pub weight: LinkWeight,
    pub link_type: PoolLinkType,
    // when this link's pool is actually subtracted (checked to gate usage regardless)
    pub condition: ChargeConditions,
    pub details: ConfigPoolLinkDetails,
}

fn identifier(name: AbilityName, variant: Variant) -> AbilityIdentifier {
    AbilityIdentifier { name, variant }
}

pub type AbilityConfigMap = IndexMap<AbilityIdentifier, AbilityConfig>;

#[derive(Debug)]
pub struct AbilityConfig {
    pub default_links: Vec<ConfigPoolLink>, // the charge pools
    pub require_presence: bool,
}

// Ability must not have multiple individual links and must not have multiple links to the same pool
pub fn default_ability_config() -> AbilityConfigMap {
    let mut map: AbilityConfigMap = IndexMap::new();

    map.insert(
        identifier(AbilityName::Gun, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Contact, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::Contact),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::CreateGroupchat, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 5,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::Contact),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::AnonymousContact, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeCondition::OnSuccess.into(),
                    details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::Contact),
                },
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeCondition::OnSuccess.into(),
                    details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                        charges: 1,
                        reset_time: 1,
                    }),
                },
            ],
        },
    );

    map.insert(
        identifier(AbilityName::FalseAnonymousContact, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeCondition::OnSuccess.into(),
                    details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::Contact),
                },
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeCondition::OnSuccess.into(),
                    details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                        charges: 1,
                        reset_time: 1,
                    }),
                },
            ],
        },
    );

    map.insert(
        identifier(AbilityName::AnonymousAnnouncement, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 2,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::FabricateLounge, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 2,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Pseudocide, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 2,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Bug, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 2,
                }),
            }],
        },
    );

    // full channel variant
    map.insert(
        identifier(AbilityName::TapIn, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    // nerfed variant
    map.insert(
        identifier(AbilityName::TapIn, 1),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::ShinigamiSacrifice, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::BackgroundCheck, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::CivilianArrest, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::UnlawfulArrest, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::UnderTheRadar, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: IterationCount::MAX,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::KiraConnection, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::AnonymousProsecute, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: IterationCount::MAX,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Autopsy, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Ipp, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::TrueNameReroll, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: IterationCount::MAX,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::PublicKidnap, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::AnonymousKidnap, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: 1,
                }),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Blackout, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                    charges: 1,
                    reset_time: IterationCount::MAX,
                }),
            }],
        },
    );

    // TrueNameReveal and NotebookReveal share one actor pool ("shinigami eyes", 2/day).
    // Both subtract on either outcome (each attempt is a use), so the pool is conditioned
    // on Success | Failure.
    map.insert(
        identifier(AbilityName::TrueNameReveal, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeConditions::all(),
                details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::ShinigamiEyes),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::NotebookReveal, 0),
        AbilityConfig {
            require_presence: false,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeConditions::all(),
                details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::ShinigamiEyes),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Prosecute, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::World(WorldChargePoolName::Prosecution),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::Outsource, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeCondition::OnSuccess.into(),
                    details: ConfigPoolLinkDetails::World(WorldChargePoolName::Prosecution),
                },
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeCondition::OnSuccess.into(),
                    details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::Invite),
                },
            ],
        },
    );

    // ForceInvite adds a player to the org outright — it can't fail, so it draws only on
    // the org's shared Invite pool (no attempts cap like TrueNameInvite needs).
    map.insert(
        identifier(AbilityName::ForceInvite, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![ConfigPoolLink {
                link_type: PoolLinkType::Restrictive,
                weight: 1,
                condition: ChargeCondition::OnSuccess.into(),
                details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::Invite),
            }],
        },
    );

    map.insert(
        identifier(AbilityName::TrueNameInvite, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeCondition::OnSuccess.into(),
                    details: ConfigPoolLinkDetails::Actor(ActorChargePoolName::Invite),
                },
                // Dedicated per-ability "attempts" pool: caps how many true-name guesses
                // the org gets, independent of the shared Invite pool. Spent on EVERY
                // guess (success or failure) — that's the whole point of an attempt cap —
                // while the shared Invite pool above is only spent on a correct guess.
                ConfigPoolLink {
                    link_type: PoolLinkType::Restrictive,
                    weight: 1,
                    condition: ChargeConditions::all(),
                    details: ConfigPoolLinkDetails::Individual(PoolSpecifier {
                        charges: 3,
                        reset_time: 1,
                    }),
                },
            ],
        },
    );

    map.insert(
        identifier(AbilityName::LeaderResign, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![],
        },
    );

    map.insert(
        identifier(AbilityName::SilentProsecute, 0),
        AbilityConfig {
            require_presence: true,
            default_links: vec![],
        },
    );

    map
}
