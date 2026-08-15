// What a credential is and what it permits.
//
// The unit of authorization is the KEY, never a person and never a connection: a key resolves to a
// privilege set, and every question the server asks is "does this set permit X". There is no
// admin-vs-player key type. When accounts arrive they will OWN keys rather than replace them, so
// nothing here has to change.
//
// The simulation/credential types (Key, Capability, ActorScope, Privileges) live in the shared
// `yagami-wire` crate -- the runtime holds the same sim state and needs the same definitions. This
// module re-exports them and adds the yagami-local LIVE pieces: the Ticket (a connection's claim on
// a key) and the KeyHandle (a key's live cancel token + issued tickets), neither of which is part of
// the simulation.

use std::collections::HashSet;

use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub use yagami_wire::{ActorScope, Capability, Key, Privileges, Token, to_flags};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ticket(Token);

impl Ticket {
    pub fn generate() -> Self {
        Self(yagami_wire::generate_token())
    }
}

impl IntoResponse for Ticket {
    fn into_response(self) -> axum::response::Response {
        self.0.into_response() // text/plain, same as when Ticket was a bare String
    }
}

// The LIVE side of a key: the handle that ties it to its connections. This is not simulation state.
// The simulation owns only the key's privileges (the raw authority, rebuilt from the accepted
// stream); the handle -- its cancel token and the tickets issued under it -- lives outside the
// simulation and is reconciled against the rebuilt key set after every rebuild, so a rewind that
// drops or keeps a key does not strand (or orphan) its connections.
pub struct KeyHandle {
    pub cancel: CancellationToken,
    pub tickets: HashSet<Ticket>,
}
