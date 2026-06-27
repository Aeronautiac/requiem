use enumflags2::{BitFlags, bitflags};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::actor::ActorDisplay;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum ChannelPermission {
    Send = 1 << 0,
    View = 1 << 1,
    LoggabilityControl = 1 << 2,
}
pub type ChannelPermissions = BitFlags<ChannelPermission>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMember {
    pub perms: ChannelPermissions,
    pub displays: IndexSet<ActorDisplay>,
}
