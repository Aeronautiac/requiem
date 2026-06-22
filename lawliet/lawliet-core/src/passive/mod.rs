use crate::ownership::OwnershipStruct;

pub use lawliet_types::passive::{ContactLogType, PassiveType};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Passive {
    pub ownership_struct: OwnershipStruct,
    pub passive_type: PassiveType,
}
