/*
* A kidnapping is a debuff + channel wrapper.
*
* A kidnapping can either be anonymous or public.
* An anonymous kidnapping does not reveal the kidnapper on release.
* A public kidnapping does.
*
* Kidnapped players may be released early by the kidnapper or a host.
*/

use crate::{ActorKey, ChannelKey};

pub use lawliet_types::kidnapping::{KidnappingType, KidnappingSource};

#[derive(Debug)]
pub struct Kidnapping {
    pub victim: ActorKey,
    pub channel_id: ChannelKey,
    pub kidnapping_type: KidnappingType,
    // the ability whose owner may release this kidnapping and whose owner is used as the
    // kidnapper for channel management (org → members added; player → player added)
    pub source: KidnappingSource,
}
