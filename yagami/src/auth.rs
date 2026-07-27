// What a credential is and what it permits.
//
// The unit of authorization is the KEY, never a person and never a connection: a key resolves to a
// privilege set, and every question the server asks is "does this set permit X". There is no
// admin-vs-player key type. When accounts arrive they will OWN keys rather than replace them, so
// nothing in this module has to change.

use std::collections::HashSet;
use std::fmt::Write as _;

use axum::response::IntoResponse;
use enumflags2::{BitFlags, bitflags};
use lawliet_types::{action::ActionActor, common::ActorKey};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub type Token = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key(Token);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ticket(Token);

impl Key {
    pub fn generate() -> Self {
        Self(generate_token())
    }
}

impl Ticket {
    pub fn generate() -> Self {
        Self(generate_token())
    }
}

impl IntoResponse for Ticket {
    fn into_response(self) -> axum::response::Response {
        self.0.into_response() // text/plain, same as when Ticket was a bare String
    }
}

// a capability is a thing a key may do that isn't "act as an actor". there is deliberately NO
// admin-vs-player key TYPE -- "admin" is just the key whose privilege set is maximal. the game only
// ever asks "does this set permit X". adding capabilities is additive.
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    // act as ActionActor::Admin, observe the System command stream, and manage keys.
    Administer = 1 << 0,
    // authority over OTHER administrators' keys. without it an admin can manage ordinary keys but
    // cannot touch a key that holds Administer -- which is what stops someone handed admin from
    // turning around and revoking it from the host who handed it to them.
    //
    // it can only ever come from someone who already holds it, and its holder cannot edit its own
    // key: authority over admins sits ABOVE admins, including above the holder's own reach.
    Supervise = 1 << 1,
}

// the wire carries a list of names rather than a bitmask so a hand-written client never has to know
// bit values; BitFlags is the in-memory representation only.
pub fn to_flags(capabilities: &[Capability]) -> BitFlags<Capability> {
    capabilities
        .iter()
        .fold(BitFlags::empty(), |flags, capability| flags | *capability)
}

// which actors a key may act as / observe. `All` is not the same as a set holding every actor that
// exists today: it covers actors created LATER, so an admin key needs no bookkeeping when the engine
// adds a player. an `Only` set is enumerated at mint time and never has to be topped up.
#[derive(Clone, Serialize, Deserialize)]
pub enum ActorScope {
    All,
    Only(HashSet<ActorKey>),
}

impl ActorScope {
    pub fn contains(&self, actor: &ActorKey) -> bool {
        match self {
            Self::All => true,
            Self::Only(actors) => actors.contains(actor),
        }
    }
}

// what a key is allowed to do. resolved from a ticket at the moment of use and never copied into the
// connection -- so narrowing or revoking a key takes effect on its live sockets immediately.
//
// Clone because a privilege CHANGE has to carry the previous set to the game task: widening is
// delivered as the difference between the two, and by the time the task handles it the ledger holds
// only the new one.
#[derive(Clone)]
pub struct Privileges {
    pub actors: ActorScope,
    pub capabilities: BitFlags<Capability>,
}

impl Privileges {
    // may a connection holding this set submit an action as this actor?
    pub fn can_act_as(&self, actor: &ActionActor) -> bool {
        match actor {
            ActionActor::Admin => self.capabilities.contains(Capability::Administer),
            // NO key may act as System, not even an admin one. System is the server's own voice and
            // it reaches machinery that exists precisely to be out of participants' hands -- an
            // admin holding it could tear down state (channels, prosecutions) that is not supposed
            // to be tearable. yagami's own System actions never arrive on a connection, so they
            // never pass through here.
            ActionActor::System => false,
            ActionActor::Player(id) => self.actors.contains(id),
            // an org is never acted AS from a connection. a player who wants an org to do something
            // sends their own player-level action, and the engine instantiates the org action from
            // it -- so the org's authority stays the engine's to grant, never a client's to claim.
            ActionActor::Organization(_) => false,
        }
    }
}

pub struct KeyData {
    pub cancel: CancellationToken,
    pub tickets: HashSet<Ticket>,
    pub privileges: Privileges,
}

pub fn generate_token() -> Token {
    let mut bytes = [0u8; 32]; // 256 bits of entropy
    getrandom::fill(&mut bytes).expect("OS CSPRNG unavailable");

    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
