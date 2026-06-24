use serde::{Deserialize, Serialize};
use specta::Type;

use crate::common::AbilityKey;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Type)]
pub enum ProsecutionSource {
    None,
    Ability(AbilityKey),
}
