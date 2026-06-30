use serde::{Deserialize, Serialize};

use crate::{common::ActorKey, role::Role};

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum AbilityName {
    Contact,
    CreateGroupchat,
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
pub enum AbilityBehaviour {
    Contact(Contact),
    Pseudocide(Pseudocide),
    Gun(Gun),
    AnonymousAnnouncement(AnonymousAnnouncement),
    AnonymousContact(AnonymousContact),
    AnonymousKidnap(AnonymousKidnap),
    PublicKidnap(PublicKidnap),
    AnonymousProsecute(AnonymousProsecute),
    Autopsy(Autopsy),
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousAnnouncement {
    pub content: String,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub target_id: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousContact {
    pub target: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Gun {
    pub target_id: ActorKey,
}

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
pub struct AnonymousKidnap {
    pub target: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct PublicKidnap {
    pub target: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousProsecute {
    pub target: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Autopsy {
    pub target: ActorKey,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Bug {
    pub target: ActorKey,
}
