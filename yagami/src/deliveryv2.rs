use std::collections::{HashMap, HashSet};

use lawliet_types::common::{ActorKey, ViewportKey};

use crate::delivery::History;

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

// delivery data transformation and batch finalization
// given some engine data, turn it into server data
// given some raw batch, turn it into the final product
// then, deliver the batch to every connection

// viewport registration needs to backfill as well
// it should be more implicit in core operations
// what we really need is some kind of viewport registration and removal system
impl DeliveryData {}
