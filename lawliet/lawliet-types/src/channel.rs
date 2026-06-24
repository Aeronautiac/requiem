use enumflags2::{BitFlags, bitflags};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::actor::ActorDisplay;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord, Serialize, Deserialize)]
pub enum ChannelPermission {
    Send = 1 << 0,
    View = 1 << 1,
}
pub type ChannelPermissions = BitFlags<ChannelPermission>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ChannelMember {
    #[specta(type = u8)]
    pub perms: ChannelPermissions,
    #[specta(type = Vec<ActorDisplay>)]
    pub displays: IndexSet<ActorDisplay>,
}
