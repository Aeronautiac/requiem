use serde::{Deserialize, Serialize};
use specta::Type;

use crate::common::AbilityKey;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Type)]
pub enum BugSource {
    Ability(AbilityKey),
    Custody,
}
