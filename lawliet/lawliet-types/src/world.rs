use serde::{Deserialize, Serialize};

// Where the world is in its life. A world exists well before anyone is playing in it: the host
// builds the roster, hands out roles, makes organizations and keys, and none of that is play.
//
// An enum rather than a "started" flag so a post-game phase can be added without turning one
// boolean into two that can disagree.
#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum WorldPhase {
    // Being built. Players may talk in whatever channels they can already see, and do nothing else.
    Setup,
    // Under way.
    Running,
}

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum WorldChargePoolName {
    Prosecution,
}

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum WorldChannelName {
    News,
    General,
    Prison,
    LAndWatari,
}

