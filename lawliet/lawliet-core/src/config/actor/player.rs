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

        // shared eye-ability pool: 2 uses per day. Only the roles with the eye abilities
        // link to it, but every player carries the pool (like Contact).
        charge_pools.insert(
            ActorChargePoolName::ShinigamiEyes,
            PoolSpecifier {
                charges: 2,
                reset_time: 1,
            },
        );

        PlayerConfig { charge_pools }
    }
}
