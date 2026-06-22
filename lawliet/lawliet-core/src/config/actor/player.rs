use indexmap::IndexMap;

use crate::{chargepool::PoolSpecifier, config::actor::ActorChargePoolName};

pub struct PlayerConfig {
    pub charge_pools: IndexMap<ActorChargePoolName, PoolSpecifier>,
}

impl PlayerConfig {
    pub fn new() -> Self {
        let mut charge_pools = IndexMap::new();

        charge_pools.insert(
            ActorChargePoolName::Contact,
            PoolSpecifier {
                charges: 5,
                reset_time: 1,
            },
        );

        PlayerConfig { charge_pools }
    }
}
