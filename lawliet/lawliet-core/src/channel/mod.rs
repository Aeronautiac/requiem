// channels are the primitive objects used to facilitate communication
//
// lounges use channels
// groups use channels
// general chat is a channel
// the news uses a channel
//
// if abilities like bug are to relay messages, or players are to read each others messages within a
// space, then those messages must be sent through channels
//
// death notes contain "private" channels within them as they still facilitate communiction between players
// any kind of log is NOT a channel as players are not allowed to speak in logs
//
// to keep memory usage low, channels will not store the messages sent through them. they are only
// used to determine what HAPPENS when a player sends a message through them
//
// messages themselves are stored in the yagami layer database and sent to lawliet for processing if
// required

use indexmap::IndexMap;

use crate::common::ActorKey;

pub use lawliet_types::channel::{ChannelMember, ChannelPermission, ChannelPermissions};

#[derive(Debug)]
pub struct Channel {
    pub loggable: bool, // whether or not abilities like autopsy can use messages sent here
    pub members: IndexMap<ActorKey, ChannelMember>, // the people in the channel and their permissions
}

impl Channel {
    pub fn new(loggable: bool) -> Self {
        Channel {
            loggable,
            members: IndexMap::new(),
        }
    }

    pub fn set_member(&mut self, id: ActorKey, settings: Option<ChannelMember>) {
        if let Some(obj) = settings {
            self.members.insert(id, obj);
        } else {
            self.members.swap_remove(&id);
        }
    }

    pub fn get_member(&self, id: ActorKey) -> Option<&ChannelMember> {
        self.members.get(&id)
    }

    pub fn set_loggable(&mut self, loggable: bool) {
        self.loggable = loggable;
    }
}
