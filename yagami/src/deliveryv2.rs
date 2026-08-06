use std::collections::{HashMap, HashSet};

use lawliet_types::common::{ActorKey, ViewportKey};

use crate::delivery::History;

// The key thing to know about delivery is that data which was sent to one connection cannot be conceptually
// unsent, with one exception (time rewind).

pub struct ViewportData {
    pub delivered_to: usize, // how much has this connection seen of this viewport?
    pub players: HashSet<ActorKey>, // which players in this connection are granting access to this viewport?
}

impl ViewportData {
    pub fn player_may_view(&self) -> bool {
        !self.players.is_empty()
    }
}

// held per connection.
// what has this connection seen?
#[derive(Default)]
pub struct DeliveryData {
    // which viewports has this connection seen,
    // and up to which point? do they still have access?
    pub viewports: HashMap<ViewportKey, ViewportData>,
}

// what do we need to do?
// backfill and live delivery
// viewport registration

// this will be built on history to some degree
// we will take in references to history in core functions
// it is possible for history to live longer than delivery data, even for a fraction of a second, so
// we cannot hold a persistent reference within the struct

// viewport registration needs to backfill as well
// it should be more implicit in core operations
// what we really need is some kind of viewport registration and removal system
impl DeliveryData {
    // lazily registers a viewport,
    pub fn enter_viewport(
        &mut self,
        history: &mut History,
        viewport: ViewportKey,
        player: ActorKey,
    ) {
    }
}
