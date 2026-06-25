use serde::{Deserialize, Serialize};

use crate::common::AbilityKey;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ProsecutionSource {
    None,
    Ability(AbilityKey),
}
