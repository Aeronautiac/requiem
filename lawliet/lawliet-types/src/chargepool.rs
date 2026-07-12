use enumflags2::{BitFlags, bitflags};
use serde::{Deserialize, Serialize};

#[derive(Hash, PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PoolLinkType {
    // all restrictive links must have charges for the ability to be usable
    Restrictive,
    // at least one permissive link must have charges for the ability to be usable
    Permissive,
}

// When an ability actually subtracts a linked charge pool: on a successful use, a
// failed use, or (both flags) always. Every linked pool is still CHECKED up front to
// gate the ability regardless of these flags — they only govern the subtraction that
// happens after the ability body returns its AbilityStatus.
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChargeCondition {
    OnSuccess = 1 << 0,
    OnFailure = 1 << 1,
}
pub type ChargeConditions = BitFlags<ChargeCondition>;
