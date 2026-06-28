pub mod modifier;
pub mod organization;
pub mod player;
pub mod state;

pub use self::organization::Organization;
pub use self::player::Player;

use crate::{
    actor::{
        modifier::{Modifier, Modifiers, Source},
        organization::LeadershipStruct,
        state::{State, States},
    },
    common::{AbilityKey, ActorKey, ChargePoolKey, NotebookKey, PassiveKey},
    config::{
        actor::{ActorChargePoolName, organization::OrganizationName},
        role::Role,
    },
};
use indexmap::{IndexMap, IndexSet};

#[derive(Hash, PartialEq, Eq, Debug, Ord, PartialOrd, Clone, Copy)]
pub enum ActorLinkType {
    Life, // if an actor has a life link to another actor, then when the main actor dies, so will
    // the other actor, and the same is true for revivals.
    Passive, // if an actor has a passive link to another actor, the main actor is treated as if it
             // has the passives of the other actor
             // it is in this order because the reverse would require a full list traversal
             // passive links are severed on death
             // link death and revive behaviours can be explicitly ignored in their corresponding actions
}

#[derive(Hash, PartialEq, Eq, Debug, Ord, PartialOrd, Clone)]
pub struct ActorLink {
    pub link_type: ActorLinkType,
    pub link_dest: ActorKey,
}

#[derive(PartialEq, Eq, Debug)]
pub enum ActorType {
    Org(Organization),
    Player(Player),
}

pub use lawliet_types::actor::ActorDisplay;
use lawliet_types::common::ChannelKey;

#[derive(Debug)]
pub struct Actor {
    pub kills: Vec<ActorKey>,
    pub abilities: IndexSet<AbilityKey>, // true ownership is in the structs themselves, these sets are here
    // for performance and utility
    // they must be synced to game state
    pub passives: IndexSet<PassiveKey>,
    pub notebooks: IndexSet<NotebookKey>, // any notebook currently HELD (not owned) by this actor
    pub modifiers: IndexMap<Source, Modifiers>,
    pub states: States,
    pub actor_type: ActorType,
    pub actor_links: IndexSet<ActorLink>,
    pub pool_map: IndexMap<ActorChargePoolName, ChargePoolKey>,
}

impl Actor {
    pub fn new_player(true_name: &str, role: Role) -> Self {
        Actor {
            kills: vec![],
            abilities: IndexSet::new(),
            passives: IndexSet::new(),
            notebooks: IndexSet::new(),
            modifiers: IndexMap::new(),
            states: States::empty(),
            actor_links: IndexSet::new(),
            actor_type: ActorType::Player(Player::new(true_name, role)),
            pool_map: IndexMap::new(),
        }
    }

    pub fn new_org(
        name: OrganizationName,
        leadership_struct: Option<LeadershipStruct>,
        channel_id: ChannelKey,
    ) -> Self {
        Actor {
            kills: vec![],
            abilities: IndexSet::new(),
            passives: IndexSet::new(),
            notebooks: IndexSet::new(),
            modifiers: IndexMap::new(),
            actor_links: IndexSet::new(),
            states: States::empty(),
            actor_type: ActorType::Org(Organization::new(name, leadership_struct, channel_id)),
            pool_map: IndexMap::new(),
        }
    }

    pub fn add_modifiers(&mut self, source: Source, modifiers: Modifiers) {
        self.modifiers.insert(source, modifiers);
    }

    pub fn remove_modifiers(&mut self, source: Source) {
        self.modifiers.swap_remove(&source);
    }

    pub fn has_modifier(&self, modifier: Modifier) -> bool {
        let mods = self.modifiers();
        mods.contains(modifier)
    }

    pub fn has_state(&self, state: State) -> bool {
        self.states.contains(state)
    }

    // adds a state
    // if any restrictions are associated with the state, it also adds the restrictions
    pub fn add_state(&mut self, new_state: State, modifiers: Modifiers) {
        self.states.set(new_state, true);
        self.add_modifiers(Source::State(new_state), modifiers);
    }

    // removes a state
    // if any restrictions are associated with the state, it removes the restrictions
    pub fn remove_state(&mut self, remove_state: State) {
        self.states.set(remove_state, false);
        self.remove_modifiers(Source::State(remove_state));
    }

    pub fn modifiers(&self) -> Modifiers {
        let mut mods = Modifiers::EMPTY;
        for modifier in self.modifiers.values() {
            mods |= *modifier;
        }
        mods
    }

    pub fn add_link(&mut self, link: ActorLink) {
        self.actor_links.insert(link);
    }

    pub fn sever_link(&mut self, link: ActorLink) {
        self.actor_links.swap_remove(&link);
    }

    pub fn remove_ability(&mut self, id: AbilityKey) {
        self.abilities.swap_remove(&id);
    }

    pub fn add_ability(&mut self, id: AbilityKey) {
        self.abilities.insert(id);
    }

    pub fn remove_passive(&mut self, id: PassiveKey) {
        self.passives.swap_remove(&id);
    }

    pub fn add_passive(&mut self, id: PassiveKey) {
        self.passives.insert(id);
    }

    pub fn add_notebook(&mut self, id: NotebookKey) {
        self.notebooks.insert(id);
    }

    pub fn remove_notebook(&mut self, id: NotebookKey) {
        self.notebooks.swap_remove(&id);
    }

    pub fn has_notebook(&self, id: NotebookKey) -> bool {
        self.notebooks.contains(&id)
    }
}

#[cfg(test)]
mod actor_tests {}
