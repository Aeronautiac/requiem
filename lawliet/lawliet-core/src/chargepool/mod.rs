pub use lawliet_types::chargepool::{ChargeCondition, ChargeConditions, PoolLinkType};

use crate::common::{ChargeCount, ChargePoolKey, IterationCount, LinkWeight};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct PoolSpecifier {
    pub charges: ChargeCount,
    pub reset_time: IterationCount,
}

#[derive(Hash, PartialEq, PartialOrd, Eq, Ord, Debug, Clone)]
pub struct PoolLink {
    pub link_type: PoolLinkType,
    pub link_dest: ChargePoolKey,
    pub weight: LinkWeight,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone)]
pub struct ChargePool {
    pub charges: ChargeCount,
    pub base_charges: ChargeCount,
    pub iterations_to_reset: IterationCount,
    pub base_reset_time: IterationCount,
    pub ref_count: u32,
}

impl ChargePool {
    pub fn new(base_charges: ChargeCount, base_reset_time: IterationCount) -> Self {
        ChargePool {
            charges: base_charges,
            base_charges,
            iterations_to_reset: 0,
            base_reset_time,
            ref_count: 0,
        }
    }

    fn use_charges(&mut self, charges: ChargeCount) {
        if charges > self.charges {
            self.charges = 0;
        } else {
            self.charges -= charges;
        }
        if self.iterations_to_reset == 0 {
            self.iterations_to_reset = self.base_reset_time;
        }
    }

    pub fn add_charges(&mut self, charges: ChargeCount) {
        self.charges += charges;
    }

    pub fn can_use(&self, link: &PoolLink) -> bool {
        self.charges >= link.weight
    }

    pub fn on_use(&mut self, link: &PoolLink) {
        self.use_charges(link.weight);
    }

    /// these are parameters because they may change throughout the game
    pub fn on_iteration(&mut self) {
        self.iterations_to_reset -= 1;
        if self.iterations_to_reset == 0 {
            self.charges = self.base_charges;
        }
    }

    pub fn on_link(&mut self) {
        self.ref_count += 1;
    }

    /// if the reference count hits zero, it returns true to signal that the pool should be
    /// destroyed (if applicable)
    pub fn on_unlink(&mut self) -> bool {
        self.ref_count -= 1;
        self.ref_count == 0
    }
}
