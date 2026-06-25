use serde::{Deserialize, Serialize};

use crate::{common::ActorKey, role::Role};

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum AbilityName {
    Contact,
    CreateGroupChat,
    AnonymousContact,
    FalseAnonymousContact,
    AnonymousAnnouncement,
    FabricateLounge,
    Pseudocide,
    Bug,
    TapIn,
    Blackout,
    ShinigamiSacrifice,
    BackgroundCheck,
    CivilianArrest,
    UnlawfulArrest,
    UnderTheRadar,
    KiraConnection,
    AnonymousProsecute,
    Autopsy,
    Ipp,
    TrueNameReroll,
    PublicKidnap,
    AnonymousKidnap,
    TrueNameReveal,
    NotebookReveal,
    Gun,
    Prosecute,
    Outsource,
    TrueNameInvite,
    LeaderResign,
    ForceInvite,
    SilentProsecute,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousAnnouncement {
    pub content: String,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousAnnouncementResponse {}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub target_id: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct ContactResponse {}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousContact {
    pub target: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousContactResponse {}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Gun {
    pub target_id: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct GunResponse {}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Pseudocide {
    pub target_id: ActorKey,
    pub true_name: String,
    pub death_message: String,
    pub role: Role,
    pub notebook_transferred: bool,
    pub ability_transferred: bool,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct PseudocideResponse {}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub enum AbilityBehaviour {
    Contact(Contact),
    Pseudocide(Pseudocide),
    Gun(Gun),
    AnonymousAnnouncement(AnonymousAnnouncement),
    AnonymousContact(AnonymousContact),
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub enum AbilityResponse {
    Contact(ContactResponse),
    Pseudocide(PseudocideResponse),
    Gun(GunResponse),
    AnonymousAnnouncement(AnonymousAnnouncementResponse),
    AnonymousContact(AnonymousContactResponse),
}
