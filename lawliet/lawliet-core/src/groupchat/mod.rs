use indexmap::{IndexSet, indexset};

use crate::common::{ActorKey, ChannelKey};

// when the owner leaves a groupchat, the owner is set to None
//
// you should not be able to give gc owner to people who dont currently have access to the gc (even
// if they are a member, they must have contact permissions)
//
// similarly to lounges, players have caches of groupchats they are in

#[derive(Debug)]
pub struct Groupchat {
    pub channel_id: ChannelKey,
    pub owner: Option<ActorKey>,
    pub members: IndexSet<ActorKey>,
}

impl Groupchat {
    pub fn new(channel_id: ChannelKey) -> Self {
        Groupchat {
            channel_id,
            owner: None,
            members: indexset! {},
        }
    }

    pub fn add_member(&mut self, id: ActorKey) {
        self.members.insert(id);
    }

    pub fn remove_member(&mut self, id: ActorKey) {
        self.members.swap_remove(&id);
    }

    pub fn contains_member(&self, id: ActorKey) -> bool {
        self.members.contains(&id)
    }

    pub fn set_owner(&mut self, owner: Option<ActorKey>) {
        self.owner = owner;
    }
}
