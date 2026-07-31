use std::rc::Rc;

use indexmap::{IndexSet, indexset};
use lawliet_types::common::{ActorKey, ChannelKey};

use crate::{
    common::{BugKey, GroupchatKey, LogID, LoungeKey},
    config::role::Role,
};

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
    pub orgs: IndexSet<ActorKey>,
    // Names this player as the sender of everything logged from them. yagami keys its message
    // store by it, which is what lets an autopsy name the real sender of something said under a
    // borrowed display.
    //
    // Written by AddPlayer, since claiming one belongs to actions rather than World.
    pub log: LogID,
}

impl Player {
    pub fn new(name: &str, role: Role) -> Self {
        let true_name = Rc::from(name);
        Player {
            role,
            true_name,
            eyes: 2,
            personal_channel_charges: 3,
            personal_channels: indexset![],
            lounges: indexset![],
            groupchats: indexset![],
            bugs: indexset![],
            orgs: indexset![],
            log: LogID::default(),
        }
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
