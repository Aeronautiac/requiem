use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Hash, PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub enum PoolLinkType {
    Limit,
    Pool,
}
