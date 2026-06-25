use serde::{Deserialize, Serialize};

use crate::common::AbilityKey;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncarcerationSource {
    None,
    Ability(AbilityKey),
}
