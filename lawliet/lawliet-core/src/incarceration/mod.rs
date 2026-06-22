/*
* An incarceration is similar to a kidnapping with two key differences:
* - incarcerated players share the Prison world channel instead of getting a private channel
* - the source is never revealed (there is no Public variant)
*
* Incarcerated players may be released by the source ability's owner or a host.
*/

use crate::ActorKey;

pub use lawliet_types::incarceration::IncarcerationSource;

#[derive(Debug)]
pub struct Incarceration {
    pub victim: ActorKey,
    pub source: IncarcerationSource,
}
