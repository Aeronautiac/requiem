use serde::{Deserialize, Serialize};
use specta::Type;

use crate::common::AbilityKey;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum IncarcerationSource {
    None,
    Ability(AbilityKey),
}
