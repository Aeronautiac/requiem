use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{
    channel::ChannelPermissions,
    common::{ActorKey, ID},
    role::Role,
};

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize, Type)]
pub enum WorldChargePoolName {
    Prosecution,
}

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize, Type)]
pub enum WorldChannelName {
    News,
    General,
    Prison,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct WorldChannelOverride {
    #[specta(type = u8)]
    pub default_perms: ChannelPermissions,
    #[specta(type = u8)]
    pub force_perms: ChannelPermissions,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash, Serialize, Deserialize, Type)]
pub enum OverrideSource {
    Role(Role),
    Manual(#[specta(type = f64)] ID),
    PressConference(ActorKey),
    Incarceration,
}
