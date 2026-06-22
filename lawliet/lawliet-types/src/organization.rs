use enumflags2::{BitFlags, bitflags};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::{common::MemberCount, role::Role};

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrganizationName {
    NULL,
    KK,
    TF,
    SPK,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum OrgAbilityPolicy {
    RequireLeader = 1 << 0,
    RequireVote = 1 << 1,
}
pub type OrgAbilityPolicies = BitFlags<OrgAbilityPolicy>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgAbility {
    pub require_roles: IndexSet<Role>,
    pub require_members: MemberCount,
    pub usage_policies: OrgAbilityPolicies,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum LeadershipTransferPolicy {
    Choose = 1 << 0,
    Random = 1 << 1,
}
pub type LeadershipTransferPolicies = BitFlags<LeadershipTransferPolicy>;
