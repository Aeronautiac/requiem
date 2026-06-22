// a lounge is simply a channel wrapper
// a lounge channel and has metadata
// creating a lounge attaches lounge specific behaviours (like no contact hiding channel
// visibilities, leaving lounges, etc...)

// lounges and group chats can be generalized to some extent
// both of them hide visibility when the nocontact modifier is enabled
// simply just have one central action that loops over both of a player's caches and manages the
// channels linked to the objects

// lounges can be fake
// if you tap into a fake lounge, your identity is revealed to the person who created the lounge,
// and you are not aware of this fact

// anonymous lounges are easy. they are just channels with different display settings for a
// particular user.

// fake lounges are harder.
// how do we handle the con artist sending messages from different perspectives?
// itd be nice for them to have a simple toggle within a single channel, but lounges would need to
// account for this
// maybe channels allow for one user to have multiple displays they can send as?
// fake lounges can simply be wrappers around this behaviour.
// this one seems the best.

// problem:
// what to display in contact logs when someone contacts someone else if multiple displays are allowed?
// solution:
// simply handle logging in the actions which create lounges. do not use lounges raw.

// players maintain an array of the lounges they are in
// leaving a lounge simply removes the lounge id from their array and subsequently removes them from
// the channel

use crate::common::ChannelKey;

pub use lawliet_types::lounge::LoungeVariant;

// storing the player's ids is necessary
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Lounge {
    pub channel_id: ChannelKey,
    pub variant: LoungeVariant,
}
