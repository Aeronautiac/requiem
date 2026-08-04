use serde::{Deserialize, Serialize};

use crate::{
    ability::{AbilityBehaviour, AbilityName},
    actor::{ActorDisplay, State},
    bug::BugSource,
    channel::{PermUpdatePolicy, ProfileBlueprint},
    chargepool::{ChargeConditions, PoolLinkType},
    command::{Command, CommandPayload, CommandRecipient},
    common::{
        AbilityKey, ActorKey, BugKey, ChannelKey, ChargeCount, ChargePoolKey, GroupchatKey,
        IncarcerationKey, IterationCount, KidnappingKey, LinkWeight, NotebookKey, PassiveKey,
        PollKey, ProfileKey, ProsecutionKey, Seed, Time, Variant,
    },
    incarceration::IncarcerationSource,
    kidnapping::{KidnappingSource, KidnappingType},
    lounge::LoungeVariant,
    organization::{LeadershipTransferPolicies, OrgAbility, OrganizationName},
    passive::PassiveType,
    poll::{
        PollOption, PollOptionIndex, PollOutcome, PollParent, PollPolicy, PollSubject, VoterPolicy,
    },
    prosecution::ProsecutionSource,
    role::Role,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum ActionError {
    EngineAlreadyInitialized,
    ActorNotFound,
    ActorIsDead,
    ActorIsAlive,
    ActorHasNotebookReceiveRestriction,
    InsufficientPermissions,
    ActorIsNotPlayer,
    NameNotUnique,
    NotebookNotFound,
    NotebookNotOwned,
    NotebookUsageBlocked,
    NotebookPassageBlocked,
    NotebookOnCooldown,
    CannotLendToYourself,
    CannotContactSelf,
    TimeAlreadyPassed,
    AbilityCategoryBlocked,
    NotEnoughMembers,
    RequiredRolesNotPresent,
    PassiveNotFound,
    AbilityConfigNotFound,
    AbilityNotFound,
    ActorIsSystem,
    AbilityNotOwned,
    AbilityMismatch,
    AbilityNotEnoughCharges,
    RoleNotImplemented,
    ItemAlreadyOwned,
    ItemAlreadyUnowned,
    ChargePoolNotFound,
    ActorIsNotOrg,
    PlayerIsNotLeader,
    PollDoesntExist,
    PollHasNoOptions,
    NotAPollOption,
    IncompatiblePolicy,
    ProfileNotFound,
    ProfileNotShareable,
    ProfileNotOwned,
    ProfileRequired,
    InvalidVoter,
    NotAVoter,
    AlreadyVoted,
    PlayerIsBlacklisted,
    OrgDoesntHaveLeadership,
    GameNotStarted,
    GameAlreadyStarted,
    ActorAlreadyInOrg,
    UserNotPresent,
    PlayerNotInOrg,
    AlreadyLeader,
    ChannelDoesntExist,
    NotAChannelMember,
    PlayerNotInLounge,
    LoungeDoesntExist,
    GroupchatDoesntExist,
    CannotContact,
    NotTheOwner,
    NotInGroupchat,
    BugNotFound,
    ProsecutionNotFound,
    AlreadyADefendant,
    NotInProsecution,
    NotACustodyPhase,
    IncompatiblePhase,
    AlreadySignalled,
    NotHoldingFloor,
    LawyerAlreadySelected,
    CannotBeOwnLawyer,
    KidnappingNotFound,
    IncarcerationNotFound,
    ActorHasStrengthenedPresence,
    PersonalChannelLimitReached,
    MustChooseSuccessor,
    NoEyes,
    CannotProsecuteSelf,
    NotAnOgMember,
    CannotSacrificeForOwnName,
    PerformerRequiresOrg,
    CannotTargetSelf,
    WorldIsBlackedOut,
    ConferenceFull,
    AlreadyInConference,
    NotInConference,
    NoNewsControl,
    AlreadyNewsAnchor,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct OrgActorInfo {
    pub org_id: ActorKey,
    pub player_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum ActionActor {
    Admin,
    System,
    Player(ActorKey),
    Organization(OrgActorInfo),
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ActionRequest {
    pub actor: ActionActor,
    pub timestamp: Time,
    pub payload: Action,
}

// ////////////////////////////////////////////////
// ABILITY //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddAbilityResponse {
    pub id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddAbility {
    pub ability_name: AbilityName,
    pub transferrable: bool,
    pub variant: Variant,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddLinkResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddLink {
    pub ability_id: AbilityKey,
    pub pool_id: ChargePoolKey,
    pub weight: LinkWeight,
    pub link_type: PoolLinkType,
    pub volatile: bool,
    pub condition: ChargeConditions,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ClearLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ClearLinks {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ClearVolatileLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ClearVolatileLinks {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveAbilityResponse {
    pub id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveAbility {
    pub ability_name: AbilityName,
    pub transferrable: bool,
    pub variant: Variant,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyAbility {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveAbility {
    pub ability_id: AbilityKey,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TakeAbilityResponse {}

// Take an ability off its owner, leaving it in the world unowned. The inverse of GiveAbility.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TakeAbility {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveLinkResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveLink {
    pub ability_id: AbilityKey,
    pub pool_id: ChargePoolKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UseAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UseAbility {
    pub ability_id: AbilityKey,
    pub ability_args: AbilityBehaviour,
}

// ////////////////////////////////////////////////
// ACTOR //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddStateResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddState {
    pub actor_id: ActorKey,
    pub state: State,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateActorLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateActorLinks {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PurgeVolatilesResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PurgeVolatiles {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveStateResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveState {
    pub actor_id: ActorKey,
    pub state: State,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SeverLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SeverLinks {
    pub actor_id: ActorKey,
}

// org

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddToOrgResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddToOrg {
    pub leader: bool,
    pub og: bool,
    pub actor_id: ActorKey,
    pub org_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ChangeOrgLeaderResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ChangeOrgLeader {
    pub org_id: ActorKey,
    pub new_leader: Option<ActorKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ResignLeadershipResponse {}

// The org's current leader steps down; the new leader is chosen per the org's
// LeadershipTransferPolicy. `chosen` is the successor named by the resigning leader,
// used only when the org's policy allows Choose.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ResignLeadership {
    pub org_id: ActorKey,
    pub chosen: Option<ActorKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveOrgAbilityResponse {
    pub id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveOrgAbility {
    pub ability_name: AbilityName,
    pub variant: Variant,
    pub org_id: ActorKey,
    pub settings: OrgAbility,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateOrgResponse {
    pub id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateOrg {
    pub name: OrganizationName,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveOrgAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveOrgAbility {
    pub org_id: ActorKey,
    pub ability_id: AbilityKey,
    pub settings: OrgAbility,
}

// news
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PressConfAccessResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PressConfAccess {
    pub target_id: ActorKey,
    pub has_access: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetNewsAnchorResponse {}

// Name the news anchor, or vacate the post with None. The anchor's kit (its abilities and passives)
// lives on the world; this hands its ownership to the target rather than remaking it, so charge
// state carries across a change of anchor. Vacating strips the kit back to ownerless.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetNewsAnchor {
    pub target_id: Option<ActorKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StartGameResponse {}

// Begin play. Until this lands the world is in Setup: it exists, it is populated, and players may
// talk in whatever channels they can already see — but nobody may use an ability or touch a
// notebook, and no clock is running.
//
// Deliberately explicit rather than something creating the world does, because setup is real work
// with no fixed length: the host is still building the roster, handing out roles and cutting keys,
// and none of that should be racing a day timer.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StartGame {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetOgStatusResponse {}

// Make an existing member an OG, or stop them being one. Separate from AddToOrg, which can only say
// what someone was at the moment they joined — this is the only way the flag ever moves afterwards.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetOgStatus {
    pub actor_id: ActorKey,
    pub org_id: ActorKey,
    pub og: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBlacklistStatusResponse {}

// Put a player on an org's blacklist, or take them off it. Blacklisting removes them if they are a
// member and stops them being added again; unblacklisting only lifts the bar, it does not readmit.
//
// A low-level primitive rather than something an org does: no ability or vote reaches it, and it is
// driven from above by whatever needs an org closed to somebody.
//
// Unlike OG status this needs no existing membership — barring somebody who was never in is the
// ordinary case.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBlacklistStatus {
    pub actor_id: ActorKey,
    pub org_id: ActorKey,
    pub blacklisted: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromOrgResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromOrg {
    pub actor_id: ActorKey,
    pub org_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetLeadershipResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetLeadership {
    pub org_id: ActorKey,
    pub policies: Option<LeadershipTransferPolicies>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SystemUseOrgAbilityResponse {
    pub poll_id: Option<PollKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SystemUseOrgAbility {
    pub org_id: ActorKey,
    pub user_id: ActorKey,
    pub ability_id: AbilityKey,
    pub ability_args: AbilityBehaviour,
    pub dont_vote: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UseOrgAbilityResponse {
    pub poll_id: Option<PollKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UseOrgAbility {
    pub org_id: ActorKey,
    pub ability_id: AbilityKey,
    pub ability_args: AbilityBehaviour,
}

// player

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddPlayerResponse {
    pub id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddPlayer {
    pub true_name: String,
    pub starting_role: Role,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveRoleResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveRole {
    pub target_id: ActorKey,
    pub role: Role,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct KillResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Kill {
    pub target_id: ActorKey,
    pub killer_id: Option<ActorKey>,
    pub death_message: Option<String>,
    pub silent: bool,
    pub allow_link_chaining: bool,
    pub sever_links: bool,
    pub set_books_dormant: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReviveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Revive {
    pub ignore_links: bool,
    pub target_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleKillResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleKill {
    pub timestamp: Time,
    pub kill: Kill,
    pub notebook_scheduled: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleReviveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleRevive {
    pub timestamp: Time,
    pub revive: Revive,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreatePersonalChannelResponse {
    pub id: ChannelKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreatePersonalChannel {}

// ////////////////////////////////////////////////
// CHARGEPOOL //
// ////////////////////////////////////////////////
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddChargePoolResponse {
    pub id: ChargePoolKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddChargePool {
    pub base_charges: ChargeCount,
    pub base_reset_time: IterationCount,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddChargesResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddCharges {
    pub id: ChargePoolKey,
    pub charges: ChargeCount,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TryDeleteChargePoolResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TryDeleteChargePool {
    pub id: ChargePoolKey,
}

// ////////////////////////////////////////////////
// COMMS //
// ////////////////////////////////////////////////

// bug

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveBugResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveBug {
    pub bug_id: BugKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateBugResponse {
    pub id: BugKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateBug {
    pub target_id: ActorKey,
    pub source: BugSource,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyBugResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyBug {
    pub bug_id: BugKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateBugVisibilitiesResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateBugVisibilities {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateContactLogViewportsResponse {}

// Recompute who is entered into the three world contact-log viewports, from effective possession of
// the matching ContactLogs passive. The record is a world singleton; this only gates who reads it.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateContactLogViewports {}

// channel

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateChannelResponse {
    pub id: ChannelKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateChannel {
    pub loggable: bool,
    // Set for a channel everyone belongs to; None for one whose membership some action owns.
    pub base_profile: Option<ProfileBlueprint>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyChannelResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
// Tearing a channel down is always archival — nothing that was said in it can be un-said, so
// there is no "really delete it" variant to select between.
pub struct DestroyChannel {
    pub channel_id: ChannelKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SendMessage {
    pub channel_id: ChannelKey,
    // Who to say it as. A player must name a profile they own, and holding Send is a property of
    // the profile rather than of the sender: the same person may be able to speak here as one of
    // their names and not as another.
    //
    // None is an admin speaking as nobody, which shows as ActorDisplay::System. It exists so the
    // host can talk in a channel without being given a name in it — nothing else has a reason to
    // send a message it holds no profile for.
    pub profile_id: Option<ProfileKey>,
    pub content: String,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetLoggableResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetLoggable {
    pub channel_id: ChannelKey,
    pub loggable: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetTrueNameResponse {}

// Set (or change) a player's true name. Updates the name index and notifies the player
// (and admin) of their current true name. Reused by initial player creation and, later, by
// name-reroll effects.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetTrueName {
    pub target_id: ActorKey,
    pub true_name: String,
}

// Membership is not set, it follows. An actor is a member of a channel exactly while they own a
// profile in it, so the actions below are the whole of joining and leaving as well as the whole of
// who may appear as what.

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddProfileResponse {
    pub profile_id: ProfileKey,
}

// Put a name into a channel. It starts owned by nobody — handing it out is SetProfileAccess, and
// keeping the two apart is what lets a profile be prepared before there is anyone to wear it.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddProfile {
    pub channel_id: ChannelKey,
    pub display: ActorDisplay,
    // False for a name the room does not know yet. It is absent from the roster until its first
    // message reveals it — see Command::ChannelRoster.
    pub visible: bool,
    // Whether more than one actor may hold it at once. An unshared profile is a name only one
    // person can be wearing, which is what makes wearing it mean anything.
    pub shared: bool,
    // Whether it could ever belong to somebody other than whoever holds it now. A name that could
    // not has no life without its holder, and is destroyed along with their membership.
    pub transferrable: bool,
    pub perm_policy: PermUpdatePolicy,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveProfileResponse {
    pub profile_id: ProfileKey,
}

// Put a name into a channel and hand it straight to somebody, which is also what makes them a
// member of it. By far the common shape: most channels give each member exactly one name of their
// own. A channel wanting something stranger — a mask nobody has been told about yet, a name several
// people wear at once — reaches for AddProfile and SetProfileAccess separately.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveProfile {
    pub channel_id: ChannelKey,
    pub player_id: ActorKey,
    pub display: ActorDisplay,
    pub visible: bool,
    pub shared: bool,
    pub transferrable: bool,
    pub perm_policy: PermUpdatePolicy,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetProfilePolicyResponse {}

// Change the rule deciding what a name permits. The new permissions follow from the next sweep.
//
// Refused if the policy cannot answer for this profile, and refused is all it is: the profile is
// left exactly as it was, still holding the rule it had. Construction is the only place an
// incompatible policy destroys anything, because there the alternative is a profile that was never
// coherent to begin with.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetProfilePolicy {
    pub channel_id: ChannelKey,
    pub profile_id: ProfileKey,
    pub perm_policy: PermUpdatePolicy,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DeleteProfileResponse {}

// Take a profile out of the channel. Whatever was said through it stays said; its owners stop
// being members if it was the last one they held.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DeleteProfile {
    pub channel_id: ChannelKey,
    pub profile_id: ProfileKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromChannelResponse {}

// Take a player out of a channel entirely.
//
// Each of their names is disposed of according to whether it could ever have belonged to anybody
// else: one that could not is destroyed, since it had no life without them, and one that could is
// simply taken off them and left in the channel for whoever comes next.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromChannel {
    pub channel_id: ChannelKey,
    pub player_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetProfileAccessResponse {}

// Give a player a profile, or take it away. Gaining a first profile makes them a member; losing
// their last one ends their membership.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetProfileAccess {
    pub channel_id: ChannelKey,
    pub profile_id: ProfileKey,
    pub player_id: ActorKey,
    pub granted: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateChannelsResponse {}

// Stamp out any base profiles that are owed, then re-evaluate every profile in the world. Trails
// every action, so nothing that moves a policy's inputs has to know that it did.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateChannels {}

// groupchat

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddToGroupchatResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddToGroupchat {
    pub groupchat_id: GroupchatKey,
    pub player_id: ActorKey,
    pub owner: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateGroupchatResponse {
    pub id: GroupchatKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateGroupchat {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromGroupchatResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromGroupchat {
    pub groupchat_id: GroupchatKey,
    pub player_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetGroupchatOwnerResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetGroupchatOwner {
    pub groupchat_id: GroupchatKey,
    pub owner: Option<ActorKey>,
}

// lounge

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateLoungeResponse {
    pub lounge_id: crate::common::LoungeKey,
    pub channel_id: ChannelKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateLounge {
    pub variant: LoungeVariant,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct LeaveLoungeResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct LeaveLounge {
    pub lounge_id: crate::common::LoungeKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromLoungeResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveFromLounge {
    pub lounge_id: crate::common::LoungeKey,
    pub player_id: ActorKey,
}

// ////////////////////////////////////////////////
// ENGINE //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct NullResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Null {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CrashResponse {}

// Debug/testing action: panics the engine on purpose so the crash path can be exercised.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Crash {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleJobResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleJob {
    pub timestamp: Time,
    pub payload: Box<Action>,
}

// ////////////////////////////////////////////////
// INCARCERATION //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateIncarcerationResponse {
    pub id: IncarcerationKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateIncarceration {
    pub victim_id: ActorKey,
    pub source: IncarcerationSource,
    // None = held indefinitely, until someone releases them. Some schedules the release, which is
    // why this can be stated here rather than needing a wrapper action to bundle the two.
    pub duration: Option<Time>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullIncarceratationsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullIncarcerations {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseIncarcerationResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseIncarceration {
    pub incarceration_id: IncarcerationKey,
    pub forced: bool,
}

// ////////////////////////////////////////////////
// KIDNAPPING //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateKidnappingResponse {
    pub id: KidnappingKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateKidnapping {
    pub victim_id: ActorKey,
    pub kidnapping_type: KidnappingType,
    pub source: KidnappingSource,
    // None = held indefinitely, until someone releases them. Some schedules the release.
    pub duration: Option<Time>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullKidnappingsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullKidnappings {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePrisonChannelResponse {}

// Reconcile who is in the prison channel against who is actually locked up.
//
// The prison hands out no seats of its own, deliberately: a channel everybody belonged to would be
// one every client had to hold and hide. So membership is managed, and the state is the authority —
// incarceration is set and cleared from more places than could each be trusted to remember this.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePrisonChannel {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateKidnappingsResponse {}

// Re-derive who is on the captors' side of every active kidnapping, and seat them.
//
// A sweep over KIDNAPPINGS rather than over channels: who the captors are is a property of what
// started the kidnapping, and that is not a question a channel can answer about itself. It is also
// the part that moves — an org's roster changes while somebody is still being held — and the rule
// deciding it will not stay as it is.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateKidnappings {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseKidnappingResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseKidnapping {
    pub kidnapping_id: KidnappingKey,
    pub forced: bool,
}

// ////////////////////////////////////////////////
// NOTEBOOK //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddNotebookResponse {
    pub id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddNotebook {
    pub fake: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveNotebookResponse {
    pub id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGiveNotebook {
    pub fake: bool,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyNotebook {
    pub notebook_id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GiveNotebook {
    pub notebook_id: NotebookKey,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetNotebookFakeResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetNotebookFake {
    pub notebook_id: NotebookKey,
    pub fake: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct LendNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct LendNotebook {
    pub notebook_id: NotebookKey,
    pub target_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct NotebookScheduledKillResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct NotebookScheduledKill {
    pub kill: Kill,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReturnBorrowedNotebooks {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReturnBorrowedNotebooksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReturnDormantBooksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReturnDormantBooks {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBooksDormantResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBooksDormant {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBorrowersToOwnersResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBorrowersToOwners {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TakeNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TakeNotebook {
    pub notebook_id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetNotebookPossessionResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetNotebookPossession {
    pub notebook_id: NotebookKey,
    pub from: Option<ActorKey>,
    pub to: Option<ActorKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct WriteNameResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct WriteName {
    pub true_name: String,
    pub death_message: Option<String>,
    pub notebook_id: NotebookKey,
    pub delay: Time,
}

// ////////////////////////////////////////////////
// PASSIVE //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddPassiveResponse {
    pub id: PassiveKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddPassive {
    pub passive_type: PassiveType,
    pub transferrable: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGivePassiveResponse {
    pub id: PassiveKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateAndGivePassive {
    pub passive_type: PassiveType,
    pub transferrable: bool,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyPassiveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyPassive {
    pub passive_id: PassiveKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GivePassiveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GivePassive {
    pub passive_id: PassiveKey,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TakePassiveResponse {}

// Take a passive off its owner, leaving it in the world unowned. The inverse of GivePassive.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TakePassive {
    pub passive_id: PassiveKey,
}

// ////////////////////////////////////////////////
// POLL //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddVoteResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddVote {
    pub poll_id: PollKey,
    pub option: PollOptionIndex,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreatePollReponse {
    pub id: PollKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreatePoll {
    pub voter_policy: VoterPolicy,
    pub parent: PollParent,
    pub subject: PollSubject,
    pub update_policy: PollPolicy,
    pub timeout_policy: PollPolicy,
    // The choices, in the order they are offered. Must not be empty.
    pub options: Vec<PollOption>,
    // Count heads instead of weight: every voter who counts is worth exactly one, and vote
    // amplification does nothing here. For ballots where being able to buy a result would be
    // absurd rather than merely strong.
    pub ignore_amplification: bool,
    pub duration: Option<Time>,
    // Who opened the poll, surfaced on the client's "vote started" notice. None = no distinct
    // opener (system-driven polls). Not stored beyond the Poll it seeds.
    pub opener: Option<ActorKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PollCleanupResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PollCleanup {
    pub poll_id: PollKey,
    // how the poll ended, so the frontend can drop it with the right resolution notice.
    pub outcome: PollOutcome,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PollTimeoutResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct PollTimeout {
    pub poll_id: PollKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveVoteResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RemoveVote {
    pub poll_id: PollKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePollsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePolls {}

// ////////////////////////////////////////////////
// PROSECUTION //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AdvanceProsecutionResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AdvanceProsecution {
    pub prosecution_id: ProsecutionKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullProsecutionsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullProsecutions {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateProsecutionsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateProsecutions {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ProsecutionVoteResResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ProsecutionVoteRes {
    pub prosecution_id: ProsecutionKey,
    pub success: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SelectLawyerResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SelectLawyer {
    pub prosecution_id: ProsecutionKey,
    pub lawyer_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetCustodyResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetCustody {
    pub defendant_id: ActorKey,
    pub custody: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SignalReadyResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SignalReady {
    pub prosecution_id: ProsecutionKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StartProsecutionResponse {
    pub id: ProsecutionKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StartProsecution {
    pub source: ProsecutionSource,
    pub prosecutor_id: ActorKey,
    pub prosecutor_display: ActorDisplay,
    pub defendant_id: ActorKey,
    pub defendant_display: ActorDisplay,
    pub autonomous: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TerminateProsecutionResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TerminateProsecution {
    pub prosecution_id: ProsecutionKey,
    // Carried straight through to CloseProsecution. Some only when the vote reached a verdict;
    // every other way a prosecution ends passes None.
    pub verdict: Option<bool>,
}

// ////////////////////////////////////////////////
// UPDATE //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Update {}

// ////////////////////////////////////////////////
// WORLD //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct NextIterationResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct NextIteration {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateOrgsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateOrgs {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct InitializeEngineResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct InitializeEngine {
    pub seed: Seed,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct InitializeWorldResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct InitializeWorld {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetRandomSeedResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetRandomSeed {
    pub seed: Seed,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateWorldViewportsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateWorldViewports {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateTimersResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateTimers {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateActorStatusesResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateActorStatuses {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePressConferenceResponse {}

// Drop any press-conference guest who can no longer be there (lost presence), so a state change that
// makes someone ineligible also takes their spot in the conference — and with it their news Send.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePressConference {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateOrgEffectiveMembersResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateOrgEffectiveMembers {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBlackoutResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetBlackout {
    pub active: bool,
}

// ////////////////////////////////////////////////
// ACTION & RESPONSE ENUMS //
// ////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    pub commands: Vec<CommandPayload>,
    // Whether we're on the mutating (execute) pass. Command pushes are suppressed on the
    // dry/validate pass, so a resolvability probe that validates an action without committing it
    // (e.g. update_polls checking a poll's payload) doesn't leak that action's commands into the
    // stream. Kept in lockstep with the `mutate` handle param by ActionExt::validate/execute.
    // Internal only — not part of the wire format.
    #[serde(skip)]
    pub mutate: bool,
}

impl ActionContext {
    pub fn push_cmd(&mut self, cmd: Command, recipient: CommandRecipient, time: Time) {
        // Dry (validate) passes must not emit commands.
        if !self.mutate {
            return;
        }
        self.commands.push(CommandPayload {
            timestamp: time,
            recipient,
            cmd,
        });
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    StartGame(StartGame),
    NextIteration(NextIteration),
    ChangeOrgLeader(ChangeOrgLeader),
    ResignLeadership(ResignLeadership),
    Kill(Kill),
    AddState(AddState),
    Revive(Revive),
    AddPlayer(AddPlayer),
    AddNotebook(AddNotebook),
    GiveNotebook(GiveNotebook),
    SetNotebookFake(SetNotebookFake),
    WriteName(WriteName),
    LendNotebook(LendNotebook),
    ScheduleKill(ScheduleKill),
    RemoveState(RemoveState),
    GiveRole(GiveRole),
    AddAbility(AddAbility),
    DestroyAbility(DestroyAbility),
    UseAbility(UseAbility),
    ScheduleRevive(ScheduleRevive),
    GiveAbility(GiveAbility),
    TakeAbility(TakeAbility),
    AddPassive(AddPassive),
    DestroyPassive(DestroyPassive),
    GivePassive(GivePassive),
    TakePassive(TakePassive),
    SeverLinks(SeverLinks),
    CreateActorLinks(CreateActorLinks),
    PurgeVolatiles(PurgeVolatiles),
    CreateAndGiveAbility(CreateAndGiveAbility),
    CreateAndGiveNotebook(CreateAndGiveNotebook),
    DestroyNotebook(DestroyNotebook),
    CreateAndGivePassive(CreateAndGivePassive),
    TakeNotebook(TakeNotebook),
    SetNotebookPossession(SetNotebookPossession),
    Null(Null),
    Crash(Crash),
    SetBorrowersToOwners(SetBorrowersToOwners),
    SetBooksDormant(SetBooksDormant),
    ReturnDormantBooks(ReturnDormantBooks),
    NotebookScheduledKill(NotebookScheduledKill),
    TryDeleteChargePool(TryDeleteChargePool),
    InitializeWorld(InitializeWorld),
    AddChargePool(AddChargePool),
    ClearVolatileLinks(ClearVolatileLinks),
    UseOrgAbility(UseOrgAbility),
    Update(Update),
    UpdatePolls(UpdatePolls),
    CreatePoll(CreatePoll),
    PollTimeout(PollTimeout),
    ScheduleJob(ScheduleJob),
    AddVote(AddVote),
    RemoveVote(RemoveVote),
    PollCleanup(PollCleanup),
    AddToOrg(AddToOrg),
    SetOgStatus(SetOgStatus),
    SetBlacklistStatus(SetBlacklistStatus),
    RemoveFromOrg(RemoveFromOrg),
    CreateOrg(CreateOrg),
    SystemUseOrgAbility(SystemUseOrgAbility),
    AddCharges(AddCharges),
    AddLink(AddLink),
    RemoveLink(RemoveLink),
    ClearLinks(ClearLinks),
    CreateOrgs(CreateOrgs),
    SetLeadership(SetLeadership),
    GiveOrgAbility(GiveOrgAbility),
    CreateAndGiveOrgAbility(CreateAndGiveOrgAbility),
    SendMessage(SendMessage),
    CreateChannel(CreateChannel),
    DestroyChannel(DestroyChannel),
    AddProfile(AddProfile),
    CreateAndGiveProfile(CreateAndGiveProfile),
    DeleteProfile(DeleteProfile),
    RemoveFromChannel(RemoveFromChannel),
    SetProfileAccess(SetProfileAccess),
    SetProfilePolicy(SetProfilePolicy),
    UpdateChannels(UpdateChannels),
    SetLoggable(SetLoggable),
    SetTrueName(SetTrueName),
    CreateLounge(CreateLounge),
    LeaveLounge(LeaveLounge),
    RemoveFromLounge(RemoveFromLounge),
    AddToGroupchat(AddToGroupchat),
    CreateGroupchat(CreateGroupchat),
    SetGroupchatOwner(SetGroupchatOwner),
    RemoveFromGroupchat(RemoveFromGroupchat),
    CreateBug(CreateBug),
    ArchiveBug(ArchiveBug),
    DestroyBug(DestroyBug),
    StartProsecution(StartProsecution),
    SetCustody(SetCustody),
    AdvanceProsecution(AdvanceProsecution),
    UpdateProsecutions(UpdateProsecutions),
    SignalReady(SignalReady),
    SelectLawyer(SelectLawyer),
    CullProsecutions(CullProsecutions),
    TerminateProsecution(TerminateProsecution),
    UpdateWorldViewports(UpdateWorldViewports),
    UpdateTimers(UpdateTimers),
    UpdateActorStatuses(UpdateActorStatuses),
    UpdatePressConference(UpdatePressConference),
    UpdateOrgEffectiveMembers(UpdateOrgEffectiveMembers),
    SetBlackout(SetBlackout),
    InitializeEngine(InitializeEngine),
    SetRandomSeed(SetRandomSeed),
    UpdateBugVisibilities(UpdateBugVisibilities),
    UpdateContactLogViewports(UpdateContactLogViewports),
    ProsecutionVoteRes(ProsecutionVoteRes),
    CreateKidnapping(CreateKidnapping),
    ReleaseKidnapping(ReleaseKidnapping),
    CullKidnappings(CullKidnappings),
    UpdateKidnappings(UpdateKidnappings),
    UpdatePrisonChannel(UpdatePrisonChannel),
    CreateIncarceration(CreateIncarceration),
    ReleaseIncarceration(ReleaseIncarceration),
    CullIncarcerations(CullIncarcerations),
    CreatePersonalChannel(CreatePersonalChannel),
    ReturnBorrowedNotebooks(ReturnBorrowedNotebooks),
    PressConfAccess(PressConfAccess),
    SetNewsAnchor(SetNewsAnchor),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActionResponse {
    StartGame(StartGameResponse),
    NextIteration(NextIterationResponse),
    CreatePersonalChannel(CreatePersonalChannelResponse),
    ChangeOrgLeader(ChangeOrgLeaderResponse),
    ResignLeadership(ResignLeadershipResponse),
    Kill(KillResponse),
    AddState(AddStateResponse),
    AddPlayer(AddPlayerResponse),
    AddNotebook(AddNotebookResponse),
    GiveNotebook(GiveNotebookResponse),
    SetNotebookFake(SetNotebookFakeResponse),
    WriteName(WriteNameResponse),
    LendNotebook(LendNotebookResponse),
    RemoveState(RemoveStateResponse),
    Revive(ReviveResponse),
    ScheduleKill(ScheduleKillResponse),
    GiveRole(GiveRoleResponse),
    AddAbility(AddAbilityResponse),
    DestroyAbility(DestroyAbilityResponse),
    GiveAbility(GiveAbilityResponse),
    TakeAbility(TakeAbilityResponse),
    UseAbility(UseAbilityResponse),
    ScheduleRevive(ScheduleReviveResponse),
    AddPassive(AddPassiveResponse),
    DestroyPassive(DestroyPassiveResponse),
    GivePassive(GivePassiveResponse),
    TakePassive(TakePassiveResponse),
    SeverLinks(SeverLinksResponse),
    CreateActorLinks(CreateActorLinksResponse),
    PurgeVolatiles(PurgeVolatilesResponse),
    CreateAndGiveAbility(CreateAndGiveAbilityResponse),
    CreateAndGiveNotebook(CreateAndGiveNotebookResponse),
    DestroyNotebook(DestroyNotebookResponse),
    CreateAndGivePassive(CreateAndGivePassiveResponse),
    TakeNotebook(TakeNotebookResponse),
    SetNotebookPossession(SetNotebookPossessionResponse),
    Null(NullResponse),
    Crash(CrashResponse),
    SetBorrowersToOwners(SetBorrowersToOwnersResponse),
    SetBooksDormant(SetBooksDormantResponse),
    ReturnDormantBooks(ReturnDormantBooksResponse),
    NotebookScheduledKill(NotebookScheduledKillResponse),
    TryDeleteChargePool(TryDeleteChargePoolResponse),
    InitializeWorld(InitializeWorldResponse),
    AddChargePool(AddChargePoolResponse),
    ClearVolatileLinks(ClearVolatileLinksResponse),
    UseOrgAbility(UseOrgAbilityResponse),
    Update(UpdateResponse),
    UpdatePolls(UpdatePollsResponse),
    CreatePoll(CreatePollReponse),
    PollTimeout(PollTimeoutResponse),
    ScheduleJob(ScheduleJobResponse),
    AddVote(AddVoteResponse),
    RemoveVote(RemoveVoteResponse),
    PollCleanup(PollCleanupResponse),
    AddToOrg(AddToOrgResponse),
    SetOgStatus(SetOgStatusResponse),
    SetBlacklistStatus(SetBlacklistStatusResponse),
    RemoveFromOrg(RemoveFromOrgResponse),
    CreateOrg(CreateOrgResponse),
    SystemUseOrgAbility(SystemUseOrgAbilityResponse),
    AddCharges(AddChargesResponse),
    AddLink(AddLinkResponse),
    RemoveLink(RemoveLinkResponse),
    ClearLinks(ClearLinksResponse),
    CreateOrgs(CreateOrgsResponse),
    SetLeadership(SetLeadershipResponse),
    GiveOrgAbility(GiveOrgAbilityResponse),
    CreateAndGiveOrgAbility(CreateAndGiveOrgAbilityResponse),
    SendMessage(SendMessageResponse),
    CreateChannel(CreateChannelResponse),
    DestroyChannel(DestroyChannelResponse),
    AddProfile(AddProfileResponse),
    CreateAndGiveProfile(CreateAndGiveProfileResponse),
    DeleteProfile(DeleteProfileResponse),
    RemoveFromChannel(RemoveFromChannelResponse),
    SetProfileAccess(SetProfileAccessResponse),
    SetProfilePolicy(SetProfilePolicyResponse),
    UpdateChannels(UpdateChannelsResponse),
    SetLoggable(SetLoggableResponse),
    SetTrueName(SetTrueNameResponse),
    CreateLounge(CreateLoungeResponse),
    LeaveLounge(LeaveLoungeResponse),
    RemoveFromLounge(RemoveFromLoungeResponse),
    AddToGroupchat(AddToGroupchatResponse),
    CreateGroupchat(CreateGroupchatResponse),
    SetGroupchatOwner(SetGroupchatOwnerResponse),
    RemoveFromGroupchat(RemoveFromGroupchatResponse),
    CreateBug(CreateBugResponse),
    ArchiveBug(ArchiveBugResponse),
    DestroyBug(DestroyBugResponse),
    StartProsecution(StartProsecutionResponse),
    SetCustody(SetCustodyResponse),
    AdvanceProsecution(AdvanceProsecutionResponse),
    UpdateProsecutions(UpdateProsecutionsResponse),
    SignalReady(SignalReadyResponse),
    SelectLawyer(SelectLawyerResponse),
    CullProsecutions(CullProsecutionsResponse),
    TerminateProsecution(TerminateProsecutionResponse),
    UpdateWorldViewports(UpdateWorldViewportsResponse),
    UpdateTimers(UpdateTimersResponse),
    UpdateActorStatuses(UpdateActorStatusesResponse),
    UpdatePressConference(UpdatePressConferenceResponse),
    UpdateOrgEffectiveMembers(UpdateOrgEffectiveMembersResponse),
    SetBlackout(SetBlackoutResponse),
    InitializeEngine(InitializeEngineResponse),
    SetRandomSeed(SetRandomSeedResponse),
    UpdateBugVisibilities(UpdateBugVisibilitiesResponse),
    UpdateContactLogViewports(UpdateContactLogViewportsResponse),
    ProsecutionVoteRes(ProsecutionVoteResResponse),
    CreateKidnapping(CreateKidnappingResponse),
    ReleaseKidnapping(ReleaseKidnappingResponse),
    CullKidnappings(CullKidnappingsResponse),
    UpdateKidnappings(UpdateKidnappingsResponse),
    UpdatePrisonChannel(UpdatePrisonChannelResponse),
    CreateIncarceration(CreateIncarcerationResponse),
    ReleaseIncarceration(ReleaseIncarcerationResponse),
    CullIncarcerations(CullIncarceratationsResponse),
    ReturnBorrowedNotebooks(ReturnBorrowedNotebooksResponse),
    PressConfAccess(PressConfAccessResponse),
    SetNewsAnchor(SetNewsAnchorResponse),
}

impl ActionActor {
    pub fn require_system(&self) -> Result<(), ActionError> {
        if self.is_system() {
            Ok(())
        } else {
            Err(ActionError::InsufficientPermissions)
        }
    }

    pub fn admin_or_system(&self) -> Result<(), ActionError> {
        if self.is_admin() || self.is_system() {
            Ok(())
        } else {
            Err(ActionError::InsufficientPermissions)
        }
    }

    pub fn player_only(&self) -> Result<(), ActionError> {
        if self.is_player() {
            Ok(())
        } else {
            Err(ActionError::ActorIsNotPlayer)
        }
    }

    pub fn org_only(&self) -> Result<(), ActionError> {
        if self.is_org() {
            Ok(())
        } else {
            Err(ActionError::ActorIsNotOrg)
        }
    }

    pub fn require_not_system(&self) -> Result<(), ActionError> {
        if self.is_system() {
            Err(ActionError::ActorIsSystem)
        } else {
            Ok(())
        }
    }

    pub fn player_or_system(&self) -> Result<(), ActionError> {
        if !self.is_player() && !self.is_system() {
            Err(ActionError::InsufficientPermissions)
        } else {
            Ok(())
        }
    }

    pub fn player_or_authoritative(&self) -> Result<(), ActionError> {
        if !self.is_player() && !self.is_authoritative() {
            Err(ActionError::InsufficientPermissions)
        } else {
            Ok(())
        }
    }

    pub fn is_player(&self) -> bool {
        matches!(self, ActionActor::Player(_))
    }

    pub fn is_system(&self) -> bool {
        matches!(self, ActionActor::System)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, ActionActor::Admin)
    }

    pub fn is_authoritative(&self) -> bool {
        self.is_admin() || self.is_system()
    }

    pub fn is_org(&self) -> bool {
        matches!(self, ActionActor::Organization(_))
    }
}
