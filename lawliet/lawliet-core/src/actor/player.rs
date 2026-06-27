use std::rc::Rc;

use indexmap::{IndexMap, IndexSet, indexset};
use lawliet_types::common::ChannelKey;

use crate::{
    ID,
    channel::ChannelPermissions,
    common::{ActorKey, BugKey, GroupchatKey, LoungeKey},
    config::{role::Role, world::WorldChannelName},
};

pub use lawliet_types::world::{OverrideSource, WorldChannelOverride};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum OverrideResolver {
    Negative,
    Positive,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SourcedWorldChannelOverride {
    pub priority: u8,
    pub data: WorldChannelOverride,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Player {
    pub role: Role,
    pub true_name: Rc<str>,
    pub eyes: u8,
    pub personal_channel_charges: u8,
    pub personal_channels: IndexSet<ChannelKey>,
    pub lounges: IndexSet<LoungeKey>,
    pub groupchats: IndexSet<GroupchatKey>,
    pub bugs: IndexSet<BugKey>, // the bugs targetting this player
    pub world_channel_overrides:
        IndexMap<WorldChannelName, IndexMap<OverrideSource, SourcedWorldChannelOverride>>,
}

// personal channels dont need some kind of world meta-structure because they can only be interacted
// with by one player and aren't really stateful outside of what channels themselves provide.
//
// the only semi-complex interaction to be aware of is loggability.
// if a player dies, the personal channel should have loggability forced to false. it
// should not be changeable until you're alive again. this is to prevent the dead from communicating
// with the living.
//
// thinking about it now, there might actually be a bug regarding world channels and death.
// if you're imprisoned, you might be able to talk in the prison channel even if you die for example.

// TODO:
// implement the set of actions for this

impl Player {
    pub fn new(name: &str, role: Role) -> Self {
        let true_name = Rc::from(name);
        Player {
            role,
            true_name,
            eyes: 2,
            personal_channel_charges: 5,
            personal_channels: indexset![],
            lounges: indexset![],
            groupchats: indexset![],
            bugs: indexset![],
            world_channel_overrides: IndexMap::new(),
        }
    }

    pub fn get_world_channel_override(
        &self,
        name: WorldChannelName,
        resolver: OverrideResolver,
    ) -> Option<WorldChannelOverride> {
        let channel_overrides = self.world_channel_overrides.get(&name)?;
        if channel_overrides.is_empty() {
            return None;
        }

        let max_priority = channel_overrides.values().map(|o| o.priority).max()?;
        let top: Vec<&WorldChannelOverride> = channel_overrides
            .values()
            .filter(|o| o.priority == max_priority)
            .map(|o| &o.data)
            .collect();

        if top.len() == 1 {
            return Some(top[0].clone());
        }

        Some(match resolver {
            OverrideResolver::Positive => WorldChannelOverride {
                default_perms: top
                    .iter()
                    .fold(ChannelPermissions::EMPTY, |acc, o| acc | o.default_perms),
                force_perms: top
                    .iter()
                    .fold(ChannelPermissions::EMPTY, |acc, o| acc | o.force_perms),
            },
            OverrideResolver::Negative => WorldChannelOverride {
                default_perms: top
                    .iter()
                    .fold(ChannelPermissions::all(), |acc, o| acc & o.default_perms),
                force_perms: top
                    .iter()
                    .fold(ChannelPermissions::all(), |acc, o| acc & o.force_perms),
            },
        })
    }

    pub fn add_lounge(&mut self, id: LoungeKey) {
        self.lounges.insert(id);
    }

    pub fn remove_lounge(&mut self, id: LoungeKey) {
        self.lounges.swap_remove(&id);
    }

    pub fn add_groupchat(&mut self, id: GroupchatKey) {
        self.groupchats.insert(id);
    }

    pub fn remove_groupchat(&mut self, id: GroupchatKey) {
        self.groupchats.swap_remove(&id);
    }

    pub fn add_bug(&mut self, id: BugKey) {
        self.bugs.insert(id);
    }

    pub fn remove_bug(&mut self, id: BugKey) {
        self.bugs.swap_remove(&id);
    }
}
