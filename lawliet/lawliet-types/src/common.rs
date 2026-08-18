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
    pub struct ViewportKey;
    pub struct TimerKey;
    // Scoped to one channel: a profile key is only meaningful alongside the ChannelKey it was
    // minted under, and every command carrying one carries that too.
    pub struct ProfileKey;
}

pub type ID = usize;
pub type Version = u64;
pub type Time = u128; // virtual units
pub type JobID = usize;
pub type Variant = u8;
pub type LinkWeight = u64;
pub type ChargeCount = u64;
pub type IterationCount = u64;
pub type PollWeight = u64;
pub type MemberCount = u64;
pub type VoteAmplifier = u64;
pub type AttemptCount = u64;
pub type Seed = u32;
pub type LogID = u16;
