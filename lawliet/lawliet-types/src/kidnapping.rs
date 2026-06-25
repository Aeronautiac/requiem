use serde::{Deserialize, Serialize};

use crate::{actor::ActorDisplay, common::AbilityKey};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KidnappingType {
    Anonymous,
    Public(ActorDisplay),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KidnappingSource {
    None,
    Ability(AbilityKey),
}
