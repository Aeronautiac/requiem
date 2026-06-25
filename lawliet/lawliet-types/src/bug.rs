use serde::{Deserialize, Serialize};

use crate::common::AbilityKey;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum BugSource {
    Ability(AbilityKey),
    Custody,
}
