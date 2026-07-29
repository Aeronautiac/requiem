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

use indexmap::{IndexMap, IndexSet};

use crate::common::{ActorKey, ViewportKey};

pub use lawliet_types::channel::{ChannelKind, ChannelMember, ChannelPermission, ChannelPermissions};

#[derive(Debug)]
pub struct Channel {
    pub loggable: bool, // whether or not abilities like autopsy can use messages sent here
    pub members: IndexMap<ActorKey, ChannelMember>, // the people in the channel and their permissions
    // Who this channel's content is delivered to. `members` above stays the authority on who is
    // in the channel and with what permissions; this is a projection of it, holding exactly the
    // members with View. Never ask the viewport whether someone can see the channel — ask the
    // channel.
    pub membership_viewport: ViewportKey,
    // What this channel is on the RECORD as having carried. The same messages, minus anything a
    // sender under Modifier::LogNullification said, and with nobody ever granted access — a
    // tap-in reads it, live delivery does not.
    //
    // Separate from membership because the two answer different questions and must be allowed to
    // disagree: the room genuinely heard an unlogged message, and monotonic state cannot take that
    // back, so suppression can only ever mean "keep it out of the record".
    pub log_viewport: ViewportKey,
}

impl Channel {
    pub fn new(
        loggable: bool,
        membership_viewport: ViewportKey,
        log_viewport: ViewportKey,
    ) -> Self {
        Channel {
            loggable,
            members: IndexMap::new(),
            membership_viewport,
            log_viewport,
        }
    }

    pub fn set_member(&mut self, id: ActorKey, settings: Option<ChannelMember>) {
        if let Some(obj) = settings {
            self.members.insert(id, obj);
        } else {
            self.members.swap_remove(&id);
        }
    }

    // The membership its viewport should hold: everyone who may read the channel.
    pub fn viewers(&self) -> IndexSet<ActorKey> {
        self.members
            .iter()
            .filter(|(_, member)| member.perms.contains(ChannelPermission::View))
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn get_member(&self, id: ActorKey) -> Option<&ChannelMember> {
        self.members.get(&id)
    }

    pub fn set_loggable(&mut self, loggable: bool) {
        self.loggable = loggable;
    }
}
