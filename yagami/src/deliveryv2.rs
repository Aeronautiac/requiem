use std::collections::{HashMap, HashSet};

use lawliet_types::common::{ActorKey, ViewportKey};

use crate::delivery::History;

// things like profiles and keys are inherently tied to the live state of a game.
// say for instance you were to create a key with access to an actor created later into a game,
// and then you were to rewind time, then create a new, different actor which happened to share that same
// actor id because it took up that free slot at a different point in time.
// this is essentially a temporal ABA problem.
// you cannot reconcile in this case, because you cannot confirm on the server level whether or not
// that is the same actor that you previously gave that key access to, nor can you throw out the data,
// because that'd require a wipe of every key's actor scope, potentially requiring the admin to
// manually modify every existing key.
//
// websocket server inputs directly modify a live game, just as engine inputs directly modify the
// engine.
//
// the proposed solution is to treat a "game" as a direct extension of the engine, and to replay not only
// engine inputs, but also server inputs on the websocket layer.
//
// then, the database storage format, networking, and server boot handling all remain exceptionally
// simple.
//
// the key invalidation scenario is resolved.
// with the new model:
// a rewind to some point in time removes any key from existence which was created for some state
// that no longer exists.
//
// there is more than one layer to a game.
// there is a harness (the raw server messiness manager), a shell (the server's extension of the engine,
// the game's data), and a core (the engine process itself)
//
// a rewind is not part of the shell layer. it is part of the harness layer.
// a key creation is part of the shell layer.
// a profile creation is part of the shell layer.
// an action request is part of the engine layer.
//
// furthermore, i've realized that a harness doesn't even need to run while nobody is connected to it,
// and given this new model, we can instantly reconstruct the server's state for that game as well
// when someone connects. all we need to do is store a cache of game keys for every game in the
// database, so connections can be handled via REST. this saves a large chunk of memory and compute.
//
// given an input, a game shell produces an output.

// data flow:
// we deliver to KEYS in general, and CONNECTIONS in specific cases
// splitting these pathways would be stupid, so what can be done is:
// one general delivery path, working on every connection.
// a piece of general data meant for one key + a piece of connection data meant for one specific
// connection.
// delivery goes through every connection, and delivers the general data regardless of if that
// connection has access to the connection specific data, but only if that connection is the one
// specified in the connection data is that part delivered as well.
//
// an connection's viewing capabilities may be expanded WHILE connected.
// in the case of an expansion, we need to go through the entire history again, and deliver anything
// that was previously missed. this is called a widening.
//
// a narrowing does not magically revoke data. this isn't how data works. instead, if the
// connection's actor scope shrunk, it goes through every viewport, and removes any lost actor from
// the players set.
// the viewport data remains. this connection has seen up to that point in the viewport. end of
// story. that cannot be taken back. in the case of a widening, this data needs to be preserved so
// data is not redelivered.
//
// we need to log every output, not just commands. a log dump is not a native output, yet it has the
// same kinds of delivery semantics.
//
// history is

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
