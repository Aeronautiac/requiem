use serde::{Deserialize, Serialize};

#[derive(Hash, PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PoolLinkType {
    Limit,
    Pool,
}
