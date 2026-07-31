use serde::{Deserialize, Serialize};

use crate::{
    common::{ActorKey, ID, LoungeKey},
    role::Role,
};

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
    Bug(Bug),
    CreateGroupchat(CreateGroupchat),
    FabricateLounge(FabricateLounge),
    FalseAnonymousContact(FalseAnonymousContact),
    Ipp(Ipp),
    Prosecute(Prosecute),
    TrueNameInvite(TrueNameInvite),
    ForceInvite(ForceInvite),
    BackgroundCheck(BackgroundCheck),
    Outsource(Outsource),
    LeaderResign(LeaderResign),
    TrueNameReveal(TrueNameReveal),
    NotebookReveal(NotebookReveal),
    CivilianArrest(CivilianArrest),
    UnlawfulArrest(UnlawfulArrest),
    UnderTheRadar(UnderTheRadar),
    ShinigamiSacrifice(ShinigamiSacrifice),
    KiraConnection(KiraConnection),
    TrueNameReroll(TrueNameReroll),
    TapIn(TapIn),
    SilentProsecute(SilentProsecute),
    Blackout(Blackout),
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Blackout {}

// Org ability. Accuse a player of being wanted, with no trial, no vote, and no announcement that
// anybody was accused at all.
//
// Right, and the target dies on the spot. Nothing rations it — the ability draws no charge pool —
// because an org that keeps being right about who is wanted is not the thing this needs to limit.
//
// Wrong, and the cost falls entirely on the accuser: they are thrown out of the org that sent them
// and barred from ever rejoining it, and the world is told who they are, true name and all, next to
// the name of the org that has just disowned them. Who they accused is never said.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct SilentProsecute {
    pub target: ActorKey,
}

// Peer into a contact channel's record by GUESSING which one.
//
// Contact channels are numbered in one strictly incrementing sequence, and that number is the only
// handle on one — you tap what you can work out from what you already know. Any contact channel is
// fair game, including one you are in yourself: the channel is told it was read but never by whom,
// so reading your own line is a way to make the other side believe an outsider is listening.
//
// Failed guesses draw a small failure pool, which is what stops the sequence being searched from
// the top down to learn how many contacts exist.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct TapIn {
    pub contact_id: ID,
}

// Open a public arrest vote against a player: any present player may vote, majority
// passes immediately, and a timeout leaves it inconclusive. On success the target is
// incarcerated for a configured duration and then automatically released.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct CivilianArrest {
    pub target: ActorKey,
}

// A civilian arrest with no vote: the target is incarcerated immediately, for a duration of its
// own. Like any incarceration the source is never disclosed — the world sees a jailing, not who
// ordered it.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct UnlawfulArrest {
    pub target: ActorKey,
}

// Self-targeted. Nothing the user says is logged for the rest of the iteration: no entry lands in
// their log viewport and no bug watching them relays anything. Carries no fields — you can only
// take yourself off the record.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct UnderTheRadar {}

// Give the target a new true name, retiring whatever anyone had learned about the old one.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct TrueNameReroll {
    pub target: ActorKey,
    // The server replaces this before the engine sees it, exactly as it does timestamps — a client
    // does not get to name itself. Not yet wired: yagami has no name reservoir to draw from, so
    // today whatever arrives is used.
    //
    // Carried INLINE rather than fetched: an engine that replays its log needs every action to be
    // self-contained, and asking a client for a name mid-action would leave a window between the
    // ask and the answer that no replay could reproduce.
    pub true_name: String,
}

// Org ability. The org spends one of its own for a name: the sacrifice dies, and the true name of
// some OTHER player is revealed to the org.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct ShinigamiSacrifice {
    // Must be an OG member of the acting org, and alive. They die for this.
    pub sacrifice: ActorKey,
    // Whose true name the org is buying. Never the sacrifice — you cannot spend someone to learn
    // their own name.
    pub name_target: ActorKey,
}

// Try to reach Kira through a lounge. The attempt always shows up in the lounge, naming the user
// outright and saying whether it found anyone — there is no way to feel for Kira quietly. What
// turns on a Kira actually being there is only whether the user's notebook block comes off.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct KiraConnection {
    pub lounge: LoungeKey,
}

// Org ability. Delegate a prosecution: invite `invitee` into the acting org and start a
// prosecution with them as the prosecutor against `defendant`.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Outsource {
    pub invitee: ActorKey,
    pub defendant: ActorKey,
}

// Org ability. The org's current leader resigns; leadership is transferred per the org's
// LeadershipTransferPolicy — Random picks an rng-chosen present member, Choose uses the
// named successor (required when the policy is Choose).
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct LeaderResign {
    pub successor: Option<ActorKey>,
}

// Privately reveal the target player's true name to the ability user (same effect as
// BackgroundCheck, a distinct ability).
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct TrueNameReveal {
    pub target: ActorKey,
}

// Privately reveal to the ability user whether the target is currently holding a notebook.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct NotebookReveal {
    pub target: ActorKey,
}

// Org ability. Invite a player into the acting org by guessing their true name; the
// guess is compared case-insensitively and the player is added immediately on a match
// (a wrong guess still spends the ability, but nothing happens).
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct TrueNameInvite {
    pub target: ActorKey,
    pub true_name: String,
}

// Org ability. Add a player into the acting org immediately, no true name required.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct ForceInvite {
    pub target: ActorKey,
}

// Privately reveal the target player's true name to the ability user.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundCheck {
    pub target: ActorKey,
}

// Openly prosecute a player: the prosecutor is shown by their real identity. Whether
// the resulting trial is autonomous is read from config (prosecution_autonomous).
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Prosecute {
    pub target: ActorKey,
}

// Like AnonymousContact, but the contactor picks which role to masquerade as instead
// of surfacing their real one. The chosen role is displayed to the contacted player.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct FalseAnonymousContact {
    pub target: ActorKey,
    pub role: Role,
}

// Grant the IPP state (strengthened presence + write immunity) to a player.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Ipp {
    pub target: ActorKey,
}

// Fabricate a lounge that, to the creator, looks like a basic lounge between two
// other players. The creator is the sole real participant but holds both players'
// displays, letting them author a conversation that never happened.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct FabricateLounge {
    pub contacted_id: ActorKey,
    pub contactor_id: ActorKey,
}

// The group-chat creation ability carries no arguments: the caller is the creator,
// and the engine makes them the owner and a member.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupchat {}

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
    // Who is publicly shown as the kidnapper. An org designates one of its own (defaults to the
    // acting member); a player is always themselves and MUST leave this None (see the handler).
    pub performer: Option<ActorKey>,
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
