use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(
    Copy, Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Ord,
    Serialize, Deserialize, EnumIter,
)]
pub enum Role {
    Kira,
    SecondKira,
    L,
    Watari,
    BeyondBirthday,
    PrivateInvestigator,
    NewsAnchor,
    Civilian,
    RogueCivilian,
    Poser,
    ConArtist,
    WantedCivilian,
    Near,
    Mello,
}
