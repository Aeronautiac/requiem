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

pub fn from_flags(capabilities: BitFlags<Capability>) -> Vec<Capability> {
    capabilities.iter().collect()
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

    // Did replacing `before` with this scope take an actor away? Added one?
    //
    // These two are what let a privilege change skip the half of the work it does not owe, so they
    // answer about REACH, not about the variants: `All` and an `Only` naming every actor alive right
    // now reach exactly the same actors, and swapping one for the other is not a change at all. That
    // is undecidable from the scopes alone -- it needs the actor list -- so the arm that straddles
    // the variants answers "maybe" as "yes". A false yes costs one wasted walk; a false no would
    // cost correctness.
    pub fn may_have_lost(&self, before: &Self) -> bool {
        match (before, self) {
            // reaches every actor there is, so nothing can have dropped out.
            (_, Self::All) => false,
            // the new set may or may not name everything the old scope reached.
            (Self::All, Self::Only(_)) => true,
            (Self::Only(before), Self::Only(after)) => !before.is_subset(after),
        }
    }

    pub fn may_have_gained(&self, before: &Self) -> bool {
        match (before, self) {
            // already reached every actor there is, so nothing can be new.
            (Self::All, _) => false,
            // the old set may or may not have named everything that exists.
            (Self::Only(_), Self::All) => true,
            (Self::Only(before), Self::Only(after)) => !after.is_subset(before),
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
    // Administer is asked about often enough, and by enough different code, to be worth a name.
    pub fn administers(&self) -> bool {
        self.capabilities.contains(Capability::Administer)
    }

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

#[cfg(test)]
mod auth_tests {
    use slotmap::KeyData;

    use super::*;

    // These two decide whether a privilege change does any work at all, so a wrong `false` is
    // silently undelivered history rather than a visible failure. The pessimistic answers are pinned
    // here deliberately: they are the ones that look like they could be tightened.

    fn actor(n: u64) -> ActorKey {
        KeyData::from_ffi(n | (1 << 32)).into()
    }

    fn only(actors: &[u64]) -> ActorScope {
        ActorScope::Only(actors.iter().copied().map(actor).collect())
    }

    #[test]
    fn an_unchanged_scope_neither_loses_nor_gains() {
        for scope in [ActorScope::All, only(&[1, 2]), only(&[])] {
            assert!(!scope.may_have_lost(&scope));
            assert!(!scope.may_have_gained(&scope));
        }
    }

    #[test]
    fn all_never_loses_and_never_gains() {
        // whatever it replaces, `All` reaches everything, so nothing dropped out...
        assert!(!ActorScope::All.may_have_lost(&only(&[1, 2])));
        // ...and whatever replaces it, `All` already reached everything, so nothing is new.
        assert!(!only(&[1, 2]).may_have_gained(&ActorScope::All));
    }

    #[test]
    fn a_swap_between_the_variants_is_answered_pessimistically() {
        // `Only` may name every actor alive, in which case neither of these is a change in reach at
        // all -- undecidable without the actor list, so both answer yes.
        assert!(only(&[1, 2]).may_have_lost(&ActorScope::All));
        assert!(ActorScope::All.may_have_gained(&only(&[1, 2])));
    }

    #[test]
    fn two_enumerated_scopes_are_answered_exactly() {
        assert!(!only(&[1, 2, 3]).may_have_lost(&only(&[1, 2]))); // grew
        assert!(only(&[1, 2, 3]).may_have_gained(&only(&[1, 2])));

        assert!(only(&[1]).may_have_lost(&only(&[1, 2]))); // shrank
        assert!(!only(&[1]).may_have_gained(&only(&[1, 2])));

        // a change can go both ways at once, which is why these are two questions and not one.
        assert!(only(&[2, 3]).may_have_lost(&only(&[1, 2])));
        assert!(only(&[2, 3]).may_have_gained(&only(&[1, 2])));
    }
}
