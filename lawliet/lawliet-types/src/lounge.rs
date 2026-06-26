use serde::{Deserialize, Serialize};

use crate::{common::ActorKey, role::Role};

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum AnonymousLoungeRoleDisplay {
    Dynamic, // based on the contactor's current role
    Static(Role),
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
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
    Anonymous {
        contacted_id: ActorKey,
        contactor_id: ActorKey,
        role_display: AnonymousLoungeRoleDisplay,
    },
}
