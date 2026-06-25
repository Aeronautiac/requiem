use serde::{Deserialize, Serialize};

use crate::{
    channel::ChannelPermissions,
    common::{ActorKey, ID},
    role::Role,
};

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum WorldChargePoolName {
    Prosecution,
}

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum WorldChannelName {
    News,
    General,
    Prison,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct WorldChannelOverride {
    pub default_perms: ChannelPermissions,
    pub force_perms: ChannelPermissions,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash, Serialize, Deserialize)]
pub enum OverrideSource {
    Role(Role),
    Manual(ID),
    PressConference(ActorKey),
    Incarceration,
}
