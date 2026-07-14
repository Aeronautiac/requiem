use serde::{Deserialize, Serialize};

use crate::{
    ability::{AbilityBehaviour, AbilityName},
    actor::{ActorDisplay, State},
    bug::BugSource,
    channel::ChannelMember,
    chargepool::{ChargeConditions, PoolLinkType},
    command::{Command, CommandPayload, CommandRecipient},
    common::{
        AbilityKey, ActorKey, BugKey, ChannelKey, ChargeCount, ChargePoolKey, GroupchatKey,
        IncarcerationKey, IterationCount, KidnappingKey, LinkWeight, NotebookKey, PassiveKey,
        PollKey, ProsecutionKey, Seed, Time, Variant,
    },
    incarceration::IncarcerationSource,
    kidnapping::{KidnappingSource, KidnappingType},
    lounge::LoungeVariant,
    organization::{LeadershipTransferPolicies, OrgAbility, OrganizationName},
    passive::PassiveType,
    poll::{PollOutcome, PollPolicy, PollSubject, PollVisibility, VoterPolicy},
    prosecution::ProsecutionSource,
    role::Role,
    world::{OverrideSource, WorldChannelName, WorldChannelOverride},
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
    InvalidVoter,
    NotAVoter,
    AlreadyVoted,
    PlayerIsBlacklisted,
    OrgDoesntHaveLeadership,
    ActorAlreadyInOrg,
    UserNotPresent,
    PlayerNotInOrg,
    AlreadyLeader,
    ChannelDoesntExist,
    NotAChannelMember,
    DisplayNotOwned,
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
    LawyerAlreadySelected,
    CannotBeOwnLawyer,
    KidnappingNotFound,
    IncarcerationNotFound,
    ActorHasStrengthenedPresence,
    PersonalChannelLimitReached,
    MustChooseSuccessor,
    NoEyes,
    CannotProsecuteSelf,
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

// channel

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateChannelResponse {
    pub id: ChannelKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreateChannel {
    pub loggable: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyChannelResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DestroyChannel {
    pub channel_id: ChannelKey,
    pub archive: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SendMessage {
    pub channel_id: ChannelKey,
    pub display: ActorDisplay,
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
pub struct SetMemberResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetMember {
    pub player_id: ActorKey,
    pub channel_id: ChannelKey,
    pub settings: Option<ChannelMember>,
}

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

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateContactChannelsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateContactChannels {
    pub player_id: ActorKey,
}

// ////////////////////////////////////////////////
// ENGINE //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DeferredCmdsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DeferredCmds {}

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

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TimedIncarcerationResponse {}

// Incarcerate a player and schedule their automatic release after `duration`. Unifies
// CreateIncarceration + a ScheduleJob(ReleaseIncarceration) into one action so it can be
// used as a single poll payload (e.g. the civilian arrest vote's accept action).
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TimedIncarceration {
    pub victim_id: ActorKey,
    pub source: IncarcerationSource,
    pub duration: Time,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePrisonChannelResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePrisonChannel {
    pub actor_id: ActorKey,
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
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullKidnappingsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CullKidnappings {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseKidnappingResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseKidnapping {
    pub kidnapping_id: KidnappingKey,
    pub forced: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateKidnapChannelsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateKidnapChannels {}

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

// ////////////////////////////////////////////////
// POLL //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddVoteResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddVote {
    pub poll_id: PollKey,
    pub accept: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreatePollReponse {
    pub id: PollKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CreatePoll {
    pub voter_policy: VoterPolicy,
    pub visibility: PollVisibility,
    pub subject: PollSubject,
    pub update_policy: PollPolicy,
    pub timeout_policy: PollPolicy,
    pub accept_payload: Box<Option<Action>>,
    pub reject_payload: Box<Option<Action>>,
    pub duration: Option<Time>,
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
pub struct UpdateProsecutionChannelsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateProsecutionChannels {
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
pub struct AddToWorldChannelsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AddToWorldChannels {
    pub player_id: ActorKey,
}

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
pub struct SetWorldChannelOverrideResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct SetWorldChannelOverride {
    pub player_id: ActorKey,
    pub channel_name: WorldChannelName,
    pub source: OverrideSource,
    pub priority: u8,
    pub override_data: Option<WorldChannelOverride>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateWorldChannelPermsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateWorldChannelPerms {
    pub player_id: ActorKey,
}

// ////////////////////////////////////////////////
// ACTION & RESPONSE ENUMS //
// ////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    pub commands: Vec<CommandPayload>,
}

impl ActionContext {
    pub fn push_cmd(&mut self, cmd: Command, recipient: CommandRecipient, time: Time) {
        self.commands.push(CommandPayload {
            timestamp: time,
            recipient,
            cmd,
        });
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    NextIteration(NextIteration),
    ChangeOrgLeader(ChangeOrgLeader),
    ResignLeadership(ResignLeadership),
    Kill(Kill),
    AddState(AddState),
    Revive(Revive),
    AddPlayer(AddPlayer),
    AddNotebook(AddNotebook),
    GiveNotebook(GiveNotebook),
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
    AddPassive(AddPassive),
    DestroyPassive(DestroyPassive),
    GivePassive(GivePassive),
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
    SetMember(SetMember),
    SetLoggable(SetLoggable),
    CreateLounge(CreateLounge),
    UpdateContactChannels(UpdateContactChannels),
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
    UpdateProsecutionChannels(UpdateProsecutionChannels),
    UpdateProsecutions(UpdateProsecutions),
    SignalReady(SignalReady),
    SelectLawyer(SelectLawyer),
    CullProsecutions(CullProsecutions),
    TerminateProsecution(TerminateProsecution),
    AddToWorldChannels(AddToWorldChannels),
    UpdateWorldChannelPerms(UpdateWorldChannelPerms),
    SetWorldChannelOverride(SetWorldChannelOverride),
    InitializeEngine(InitializeEngine),
    SetRandomSeed(SetRandomSeed),
    DeferredCmds(DeferredCmds),
    UpdateBugVisibilities(UpdateBugVisibilities),
    ProsecutionVoteRes(ProsecutionVoteRes),
    CreateKidnapping(CreateKidnapping),
    ReleaseKidnapping(ReleaseKidnapping),
    CullKidnappings(CullKidnappings),
    UpdateKidnapChannels(UpdateKidnapChannels),
    UpdatePrisonChannel(UpdatePrisonChannel),
    CreateIncarceration(CreateIncarceration),
    ReleaseIncarceration(ReleaseIncarceration),
    TimedIncarceration(TimedIncarceration),
    CullIncarcerations(CullIncarcerations),
    CreatePersonalChannel(CreatePersonalChannel),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActionResponse {
    NextIteration(NextIterationResponse),
    CreatePersonalChannel(CreatePersonalChannelResponse),
    ChangeOrgLeader(ChangeOrgLeaderResponse),
    ResignLeadership(ResignLeadershipResponse),
    Kill(KillResponse),
    AddState(AddStateResponse),
    AddPlayer(AddPlayerResponse),
    AddNotebook(AddNotebookResponse),
    GiveNotebook(GiveNotebookResponse),
    WriteName(WriteNameResponse),
    LendNotebook(LendNotebookResponse),
    RemoveState(RemoveStateResponse),
    Revive(ReviveResponse),
    ScheduleKill(ScheduleKillResponse),
    GiveRole(GiveRoleResponse),
    AddAbility(AddAbilityResponse),
    DestroyAbility(DestroyAbilityResponse),
    GiveAbility(GiveAbilityResponse),
    UseAbility(UseAbilityResponse),
    ScheduleRevive(ScheduleReviveResponse),
    AddPassive(AddPassiveResponse),
    DestroyPassive(DestroyPassiveResponse),
    GivePassive(GivePassiveResponse),
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
    SetMember(SetMemberResponse),
    SetLoggable(SetLoggableResponse),
    CreateLounge(CreateLoungeResponse),
    UpdateContactChannels(UpdateContactChannelsResponse),
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
    UpdateProsecutionChannels(UpdateProsecutionChannelsResponse),
    UpdateProsecutions(UpdateProsecutionsResponse),
    SignalReady(SignalReadyResponse),
    SelectLawyer(SelectLawyerResponse),
    CullProsecutions(CullProsecutionsResponse),
    TerminateProsecution(TerminateProsecutionResponse),
    AddToWorldChannels(AddToWorldChannelsResponse),
    UpdateWorldChannelPerms(UpdateWorldChannelPermsResponse),
    SetWorldChannelOverride(SetWorldChannelOverrideResponse),
    InitializeEngine(InitializeEngineResponse),
    SetRandomSeed(SetRandomSeedResponse),
    DeferredCmds(DeferredCmdsResponse),
    UpdateBugVisibilities(UpdateBugVisibilitiesResponse),
    ProsecutionVoteRes(ProsecutionVoteResResponse),
    CreateKidnapping(CreateKidnappingResponse),
    ReleaseKidnapping(ReleaseKidnappingResponse),
    CullKidnappings(CullKidnappingsResponse),
    UpdateKidnapChannels(UpdateKidnapChannelsResponse),
    UpdatePrisonChannel(UpdatePrisonChannelResponse),
    CreateIncarceration(CreateIncarcerationResponse),
    ReleaseIncarceration(ReleaseIncarcerationResponse),
    TimedIncarceration(TimedIncarcerationResponse),
    CullIncarcerations(CullIncarceratationsResponse),
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
