use serde::{Deserialize, Serialize};
use specta::Type;

use crate::common::ActorKey;

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub enum LoungeVariant {
    Fake {
        creator_id: ActorKey,
        contacted_id: ActorKey,
        contactor_id: ActorKey,
    },
    Basic {
        contacted_id: ActorKey,
        contactor_id: ActorKey,
    },
}
