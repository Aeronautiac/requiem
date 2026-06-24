use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{actor::ActorDisplay, common::AbilityKey};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum KidnappingType {
    Anonymous,
    Public(ActorDisplay),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum KidnappingSource {
    None,
    Ability(AbilityKey),
}
