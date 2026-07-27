use crate::{common::ViewportKey, ownership::OwnershipStruct};

pub use lawliet_types::passive::{ContactEvent, ContactLog, ContactLogType, PassiveType};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Passive {
    pub ownership_struct: OwnershipStruct,
    pub passive_type: PassiveType,
    // Who this passive's accumulated log will be addressed to. ALLOCATED BUT DELIBERATELY
    // EMPTY: nothing is addressed here yet, because AddContactLog — the command this exists
    // for — is still unbuilt (its payload is commented out).
    //
    // Membership is left unwired on purpose rather than guessed at. Access to a contact log is
    // gated on EFFECTIVE possession, not ownership: an actor with an ActorLinkType::Passive
    // link inherits another's passives, so Watari holding the ability means L can read the logs
    // too. `actor_get_effective_passive` answers that question by passive TYPE and returns only
    // the first match, so it cannot be inverted into "who reaches passive P" while two passives
    // of a type can coexist. Doing it right needs a reachability helper plus a recompute pass
    // with the same triggers UpdateBugVisibilities has (link create/sever, DisablePassiveLinks,
    // state changes) — build that alongside the contact log, not before it.
    pub viewport: ViewportKey,
}
