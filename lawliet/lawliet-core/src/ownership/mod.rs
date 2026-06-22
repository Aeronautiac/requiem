use crate::common::ActorKey;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct OwnershipStruct {
    pub owner: Option<ActorKey>, // the actor which this item is owned by (if any)
    pub volatile: bool,    // determines whether or not the item is deleted when the owner changes
    // significantly (i.e., the role changes)
    pub transferrable: bool, // determines whether or not the ability will transfer on death (on
                             // transfer, the item will no longer be volatile)
}

impl OwnershipStruct {
    pub fn new(transferrable: bool) -> Self {
        OwnershipStruct {
            owner: None,
            volatile: false,
            transferrable,
        }
    }

    pub fn set_owner(&mut self, id: ActorKey, volatile: bool) {
        self.owner = Some(id);
        self.volatile = volatile;
    }
}
