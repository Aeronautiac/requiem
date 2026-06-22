pub mod organization;
pub mod player;

// define base actor states
#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum ActorChargePoolName {
    Contact,
    Invite,
}
