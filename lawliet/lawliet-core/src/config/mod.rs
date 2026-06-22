pub mod ability;
pub mod actor;
pub mod defaults;
pub mod role;
pub mod ruleset;
pub mod state;
pub mod world;

use crate::config::{
    ability::{AbilityConfigMap, default_ability_config},
    actor::{
        organization::{OrganizationConfigMap, default_org_config},
        player::PlayerConfig,
    },
    defaults::{DefaultConfig, default_defaults},
    role::{RoleConfigMap, default_role_config},
    state::{StateModifierMap, default_state_modifiers},
    world::WorldConfig,
};

// these should be maps
pub struct Config {
    pub roles: RoleConfigMap,
    pub abilities: AbilityConfigMap,
    pub state_modifiers: StateModifierMap,
    pub defaults: DefaultConfig,
    pub world_config: WorldConfig,
    pub player_config: PlayerConfig,
    pub org_config: OrganizationConfigMap,
}

impl Config {
    pub fn new() -> Self {
        Config {
            roles: default_role_config(),
            abilities: default_ability_config(),
            state_modifiers: default_state_modifiers(),
            defaults: default_defaults(), // defaults are things like fallback death messages
            world_config: WorldConfig::new(),
            player_config: PlayerConfig::new(),
            org_config: default_org_config(),
        }
    }
}
