pub mod organization;
pub mod player;

// define base actor states
#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum ActorChargePoolName {
    Contact,
    Invite,
    // shared "shinigami eyes" ability pool (TrueNameReveal / NotebookReveal). Distinct
    // from a player's `eyes` count, which is a separate permanent/volatile resource.
    ShinigamiEyes,
}
