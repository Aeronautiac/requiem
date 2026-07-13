use indexmap::IndexMap;

use crate::{
    actor::organization::{
        LeadershipTransferPolicies, LeadershipTransferPolicy, OrgAbilityPolicies, OrgAbilityPolicy,
    },
    chargepool::PoolSpecifier,
    common::MemberCount,
    config::{
        ability::{AbilityIdentifier, AbilityName},
        actor::ActorChargePoolName,
        role::Role,
    },
    passive::PassiveType,
};

pub use lawliet_types::organization::OrganizationName;

#[derive(Clone)]
pub struct OrgConfigAbility {
    pub identifier: AbilityIdentifier,
    pub require_roles: Vec<Role>,
    pub require_members: MemberCount,
    pub usage_policies: OrgAbilityPolicies,
}

pub struct OrgLeadershipConfig {
    pub transfer_policies: LeadershipTransferPolicies,
}

pub struct OrganizationConfig {
    pub leadership: Option<OrgLeadershipConfig>,
    pub abilities: Vec<OrgConfigAbility>,
    pub passives: Vec<PassiveType>,
    // Charge pools created on the org actor at creation (before its abilities, which link
    // to them). Orgs' invite abilities (ForceInvite/TrueNameInvite/Outsource) all draw on
    // the shared Invite pool.
    pub charge_pools: IndexMap<ActorChargePoolName, PoolSpecifier>,
}

pub type OrganizationConfigMap = IndexMap<OrganizationName, OrganizationConfig>;

// The default set of org charge pools: a shared Invite pool, one invite per day.
fn org_charge_pools() -> IndexMap<ActorChargePoolName, PoolSpecifier> {
    let mut pools = IndexMap::new();
    pools.insert(
        ActorChargePoolName::Invite,
        PoolSpecifier {
            charges: 1,
            reset_time: 1,
        },
    );
    pools
}

pub fn default_org_config() -> OrganizationConfigMap {
    let mut map = IndexMap::new();

    map.insert(
        OrganizationName::NULL,
        OrganizationConfig {
            charge_pools: org_charge_pools(),
            leadership: None,
            abilities: vec![],
            passives: vec![],
        },
    );

    map.insert(
        OrganizationName::KK,
        OrganizationConfig {
            charge_pools: org_charge_pools(),
            leadership: None,
            abilities: vec![
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::TapIn,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 4,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Blackout,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 5,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::PublicKidnap,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 3,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousKidnap,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 5,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::AnonymousKidnap,
                        variant: 0,
                    },
                    require_roles: vec![Role::Kira, Role::SecondKira],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::ShinigamiSacrifice,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::ForceInvite,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
            ],
            passives: vec![],
        },
    );

    map.insert(
        OrganizationName::TF,
        OrganizationConfig {
            charge_pools: org_charge_pools(),
            leadership: Some(OrgLeadershipConfig {
                transfer_policies: LeadershipTransferPolicy::Random.into(),
            }),
            abilities: vec![
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::BackgroundCheck,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 3,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::CivilianArrest,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 4,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::UnlawfulArrest,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 5,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::UnlawfulArrest,
                        variant: 0,
                    },
                    require_roles: vec![Role::PrivateInvestigator, Role::Watari],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::LeaderResign,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireLeader.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Prosecute,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::empty(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Outsource,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::empty(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::TrueNameInvite,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireLeader.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::SilentProsecute,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::empty(),
                },
            ],
            passives: vec![],
        },
    );

    map.insert(
        OrganizationName::SPK,
        OrganizationConfig {
            charge_pools: org_charge_pools(),
            leadership: Some(OrgLeadershipConfig {
                transfer_policies: LeadershipTransferPolicy::Random.into(),
            }),
            abilities: vec![
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::BackgroundCheck,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 3,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::CivilianArrest,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 4,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::UnlawfulArrest,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 5,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::UnlawfulArrest,
                        variant: 0,
                    },
                    require_roles: vec![Role::PrivateInvestigator, Role::Watari],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::LeaderResign,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireLeader.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Prosecute,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::empty(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::Outsource,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::empty(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::TrueNameInvite,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireLeader.into(),
                },
                OrgConfigAbility {
                    identifier: AbilityIdentifier {
                        name: AbilityName::SilentProsecute,
                        variant: 0,
                    },
                    require_roles: vec![],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::empty(),
                },
            ],
            passives: vec![],
        },
    );

    map
}
