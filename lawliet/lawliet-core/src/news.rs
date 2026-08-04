use indexmap::IndexSet;

use crate::common::{AbilityKey, ActorKey, PassiveKey};

// The news, as one thing on the world: who runs it, the kit that being the anchor grants, and the
// guests the anchor has let speak.
//
// News anchor is a status, not a role — the same way being in Kira's Kingdom is not a role. The kit
// (its abilities and passives) is created once, ownerless, on world init, and its ownership is
// REASSIGNED to each successive anchor rather than destroyed and remade — so charge state survives a
// change of anchor. The kit is non-volatile and non-transferrable: a role change never purges it and
// a killer never inherits it. It simply waits on whoever last held it until the host hands it on.
#[derive(Debug)]
pub struct News {
    // Who currently holds the anchor post, or None when it is vacant.
    pub anchor: Option<ActorKey>,
    // The anchor's kit, held here so it can be handed on rather than remade. NewsControl (managing
    // the conference) and NewsAccess (speaking on the news) live among the passives.
    pub anchor_abilities: IndexSet<AbilityKey>,
    pub anchor_passives: IndexSet<PassiveKey>,
    // Players the anchor has let speak on the news beyond themselves. Speaking on the news is granted
    // by holding NewsAccess or by being in here (see NewsPolicy).
    pub press_conf: IndexSet<ActorKey>,
}

impl News {
    pub fn new() -> Self {
        News {
            anchor: None,
            anchor_abilities: IndexSet::new(),
            anchor_passives: IndexSet::new(),
            press_conf: IndexSet::new(),
        }
    }
}
