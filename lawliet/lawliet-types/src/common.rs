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
