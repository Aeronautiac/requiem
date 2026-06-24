use slotmap::new_key_type;

new_key_type! {
    pub struct ActorKey;
    pub struct AbilityKey;
    pub struct PassiveKey;
    pub struct NotebookKey;
    pub struct ChannelKey;
    pub struct ChargePoolKey;
    pub struct PollKey;
    pub struct LoungeKey;
    pub struct GroupchatKey;
    pub struct BugKey;
    pub struct ProsecutionKey;
    pub struct KidnappingKey;
    pub struct IncarcerationKey;
}

// Slotmap keys are opaque numeric identifiers on the frontend.
// They serialize as u64 via serde. We delegate to f64 (also `number` in TS) to avoid
// specta-typescript's BigInt-forbidden restriction on u64.
macro_rules! impl_key_specta {
    ($($key:ty),* $(,)?) => {$(
        impl specta::Type for $key {
            fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
                f64::definition(types)
            }
        }
    )*};
}

impl_key_specta!(
    ActorKey, AbilityKey, PassiveKey, NotebookKey, ChannelKey,
    ChargePoolKey, PollKey, LoungeKey, GroupchatKey, BugKey,
    ProsecutionKey, KidnappingKey, IncarcerationKey,
);

pub type ID = usize; // host-inserted frontend identifiers (e.g. OverrideSource::Manual)
pub type Version = u8;
pub type Time = u128; // intended to be used as unix time in milliseconds
pub type JobID = usize;
pub type Variant = u8;
pub type LinkWeight = u16;
pub type ChargeCount = u16;
pub type IterationCount = u8;
pub type PollWeight = u16;
pub type MemberCount = u16;
pub type VoteAmplifier = u16;
pub type AttemptCount = u16;
pub type Seed = u32;
