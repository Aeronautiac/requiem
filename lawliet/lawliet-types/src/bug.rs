use serde::{Deserialize, Serialize};

use crate::common::AbilityKey;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum BugSource {
    Ability(AbilityKey),
    Custody,
}

// Target-facing bug context: what a bugged player is told about *why* they're bugged.
// Deliberately coarser than BugSource — it omits the owner-identifying AbilityKey so the
// target learns they're under surveillance and in what context, but never who planted it.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum BugContext {
    // planted explicitly via a bug ability
    Explicit,
    // incidental to being held in custody
    Custody,
}

impl From<BugSource> for BugContext {
    fn from(source: BugSource) -> Self {
        match source {
            BugSource::Ability(_) => BugContext::Explicit,
            BugSource::Custody => BugContext::Custody,
        }
    }
}
