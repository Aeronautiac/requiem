use crate::ownership::OwnershipStruct;

pub use lawliet_types::passive::{ContactEvent, ContactLog, ContactLogType, PassiveType};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Passive {
    pub ownership_struct: OwnershipStruct,
    pub passive_type: PassiveType,
    // A passive owns no viewport. The one passive type that feeds a viewport — ContactLogs — reads
    // one of three WORLD-level log viewports instead (see World::contact_log_viewports). Holding
    // the passive is what enters an actor into the matching one, via UpdatePassiveVisibilities.
}
