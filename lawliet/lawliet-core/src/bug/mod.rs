use crate::common::ActorKey;

//   when someone gains a bug ability, they should also gain access to every log the
//   ability has created. simply holding expired bug entries in memory is the simplest way to handle
//   this. without this, we'd need complex id tracking and ability specific metadata. bug entries
//   are tiny anyway.
//   prosecutions on the other hand can be safely deleted because they have no interactions outside
//   of their standard lifecycle.
// - implement custody bugs

pub use lawliet_types::bug::BugSource;

#[derive(Debug)]
pub struct Bug {
    pub target_id: ActorKey,
    pub source: BugSource,
    pub enabled: bool,
}

impl Bug {
    pub fn new(target_id: ActorKey, source: BugSource) -> Self {
        Bug {
            target_id,
            source,
            enabled: true,
        }
    }
}
