use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{
    ability::{AbilityBehaviour, AbilityName},
    actor::{ActorDisplay, State},
    bug::BugSource,
    channel::ChannelMember,
    chargepool::PoolLinkType,
    command::{Command, CommandPayload},
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
    poll::{PollPolicy, PollVisibility, VoterPolicy},
    prosecution::ProsecutionSource,
    role::Role,
    world::{OverrideSource, WorldChannelName, WorldChannelOverride},
};

#[derive(Debug, Serialize, Deserialize, Type)]
pub enum ActionError {
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
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct OrgActorInfo {
    pub org_id: ActorKey,
    pub player_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub enum ActionActor {
    Admin,
    System,
    Player(ActorKey),
    Organization(OrgActorInfo),
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ActionRequest {
    pub actor: ActionActor,
    #[specta(type = f64)]
    pub timestamp: Time,
    pub payload: Action,
}

// ////////////////////////////////////////////////
// ABILITY //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddAbilityResponse {
    pub id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddAbility {
    pub ability_name: AbilityName,
    pub transferrable: bool,
    pub variant: Variant,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddLinkResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddLink {
    pub ability_id: AbilityKey,
    pub pool_id: ChargePoolKey,
    pub weight: LinkWeight,
    pub link_type: PoolLinkType,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ClearLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ClearLinks {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ClearVolatileLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ClearVolatileLinks {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGiveAbilityResponse {
    pub id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGiveAbility {
    pub ability_name: AbilityName,
    pub transferrable: bool,
    pub variant: Variant,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyAbility {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveAbility {
    pub ability_id: AbilityKey,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveLinkResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveLink {
    pub ability_id: AbilityKey,
    pub pool_id: ChargePoolKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UseAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UseAbility {
    pub ability_id: AbilityKey,
    pub ability_args: AbilityBehaviour,
}

// ////////////////////////////////////////////////
// ACTOR //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddStateResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddState {
    pub actor_id: ActorKey,
    pub state: State,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateActorLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateActorLinks {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct PurgeVolatilesResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct PurgeVolatiles {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveStateResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveState {
    pub actor_id: ActorKey,
    pub state: State,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SeverLinksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SeverLinks {
    pub actor_id: ActorKey,
}

// org

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddToOrgResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddToOrg {
    pub leader: bool,
    pub og: bool,
    pub actor_id: ActorKey,
    pub org_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ChangeOrgLeaderResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ChangeOrgLeader {
    pub org_id: ActorKey,
    pub new_leader: Option<ActorKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGiveOrgAbilityResponse {
    pub id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGiveOrgAbility {
    pub ability_name: AbilityName,
    pub variant: Variant,
    pub org_id: ActorKey,
    pub settings: OrgAbility,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateOrgResponse {
    pub id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateOrg {
    pub name: OrganizationName,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveOrgAbilityResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveOrgAbility {
    pub org_id: ActorKey,
    pub ability_id: AbilityKey,
    pub settings: OrgAbility,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveFromOrgResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveFromOrg {
    pub actor_id: ActorKey,
    pub org_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetLeadershipResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetLeadership {
    pub org_id: ActorKey,
    #[specta(type = Option<u8>)]
    pub policies: Option<LeadershipTransferPolicies>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SystemUseOrgAbilityResponse {
    pub poll_id: Option<PollKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SystemUseOrgAbility {
    pub org_id: ActorKey,
    pub user_id: ActorKey,
    pub ability_id: AbilityKey,
    pub ability_args: AbilityBehaviour,
    pub dont_vote: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UseOrgAbilityResponse {
    pub poll_id: Option<PollKey>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UseOrgAbility {
    pub org_id: ActorKey,
    pub ability_id: AbilityKey,
    pub ability_args: AbilityBehaviour,
}

// player

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddPlayerResponse {
    pub id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddPlayer {
    pub true_name: String,
    pub starting_role: Role,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveRoleResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveRole {
    pub target_id: ActorKey,
    pub role: Role,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct KillResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct Kill {
    pub target_id: ActorKey,
    pub killer_id: Option<ActorKey>,
    pub death_message: Option<String>,
    pub silent: bool,
    pub allow_link_chaining: bool,
    pub sever_links: bool,
    pub set_books_dormant: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ReviveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct Revive {
    pub ignore_links: bool,
    pub target_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ScheduleKillResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ScheduleKill {
    #[specta(type = f64)]
    pub timestamp: Time,
    pub kill: Kill,
    pub notebook_scheduled: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ScheduleReviveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ScheduleRevive {
    #[specta(type = f64)]
    pub timestamp: Time,
    pub revive: Revive,
}

// ////////////////////////////////////////////////
// CHARGEPOOL //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddChargePoolResponse {
    pub id: ChargePoolKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddChargePool {
    pub base_charges: ChargeCount,
    pub base_reset_time: IterationCount,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddChargesResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddCharges {
    pub id: ChargePoolKey,
    pub charges: ChargeCount,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct TryDeleteChargePoolResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct TryDeleteChargePool {
    pub id: ChargePoolKey,
}

// ////////////////////////////////////////////////
// COMMS //
// ////////////////////////////////////////////////

// bug

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ArchiveBugResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ArchiveBug {
    pub bug_id: BugKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateBugResponse {
    pub id: BugKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateBug {
    pub target_id: ActorKey,
    pub source: BugSource,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyBugResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyBug {
    pub bug_id: BugKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateBugVisibilitiesResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateBugVisibilities {}

// channel

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateChannelResponse {
    pub id: ChannelKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateChannel {
    pub loggable: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyChannelResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyChannel {
    pub channel_id: ChannelKey,
    pub archive: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SendMessageResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SendMessage {
    pub channel_id: ChannelKey,
    pub display: ActorDisplay,
    pub content: String,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetLoggableResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetLoggable {
    pub channel_id: ChannelKey,
    pub loggable: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetMemberResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetMember {
    pub player_id: ActorKey,
    pub channel_id: ChannelKey,
    pub settings: Option<ChannelMember>,
}

// groupchat

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddToGroupchatResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddToGroupchat {
    pub groupchat_id: GroupchatKey,
    pub player_id: ActorKey,
    pub owner: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateGroupchatResponse {
    pub id: GroupchatKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateGroupchat {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveFromGroupchatResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveFromGroupchat {
    pub groupchat_id: GroupchatKey,
    pub player_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetGroupchatOwnerResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetGroupchatOwner {
    pub groupchat_id: GroupchatKey,
    pub owner: Option<ActorKey>,
}

// lounge

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateLoungeResponse {
    pub lounge_id: crate::common::LoungeKey,
    pub channel_id: ChannelKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateLounge {
    pub variant: LoungeVariant,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct LeaveLoungeResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct LeaveLounge {
    pub lounge_id: crate::common::LoungeKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveFromLoungeResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveFromLounge {
    pub lounge_id: crate::common::LoungeKey,
    pub player_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateContactChannelsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateContactChannels {
    pub player_id: ActorKey,
}

// ////////////////////////////////////////////////
// ENGINE //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DeferredCmdsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DeferredCmds {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct NullResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct Null {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ScheduleJobResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ScheduleJob {
    #[specta(type = f64)]
    pub timestamp: Time,
    pub payload: Box<Action>,
}

// ////////////////////////////////////////////////
// INCARCERATION //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateIncarcerationResponse {
    pub id: IncarcerationKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateIncarceration {
    pub victim_id: ActorKey,
    pub source: IncarcerationSource,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CullIncarceratationsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CullIncarcerations {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ReleaseIncarcerationResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ReleaseIncarceration {
    pub incarceration_id: IncarcerationKey,
    pub forced: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdatePrisonChannelResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdatePrisonChannel {
    pub actor_id: ActorKey,
}

// ////////////////////////////////////////////////
// KIDNAPPING //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateKidnappingResponse {
    pub id: KidnappingKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateKidnapping {
    pub victim_id: ActorKey,
    pub kidnapping_type: KidnappingType,
    pub source: KidnappingSource,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CullKidnappingsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CullKidnappings {
    pub ability_id: AbilityKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ReleaseKidnappingResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ReleaseKidnapping {
    pub kidnapping_id: KidnappingKey,
    pub forced: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateKidnapChannelsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateKidnapChannels {}

// ////////////////////////////////////////////////
// NOTEBOOK //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddNotebookResponse {
    pub id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddNotebook {
    pub fake: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGiveNotebookResponse {
    pub id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGiveNotebook {
    pub fake: bool,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyNotebook {
    pub notebook_id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GiveNotebook {
    pub notebook_id: NotebookKey,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct LendNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct LendNotebook {
    pub notebook_id: NotebookKey,
    pub target_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct NotebookScheduledKillResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct NotebookScheduledKill {
    pub kill: Kill,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ReturnDormantBooksResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ReturnDormantBooks {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetBooksDormantResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetBooksDormant {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetBorrowersToOwnersResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetBorrowersToOwners {
    pub actor_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct TakeNotebookResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct TakeNotebook {
    pub notebook_id: NotebookKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct WriteNameResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct WriteName {
    pub true_name: String,
    pub death_message: Option<String>,
    pub notebook_id: NotebookKey,
    #[specta(type = f64)]
    pub delay: Time,
}

// ////////////////////////////////////////////////
// PASSIVE //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddPassiveResponse {
    pub id: PassiveKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddPassive {
    pub passive_type: PassiveType,
    pub transferrable: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGivePassiveResponse {
    pub id: PassiveKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateAndGivePassive {
    pub passive_type: PassiveType,
    pub transferrable: bool,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyPassiveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct DestroyPassive {
    pub passive_id: PassiveKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GivePassiveResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct GivePassive {
    pub passive_id: PassiveKey,
    pub actor_id: ActorKey,
    pub volatile: bool,
}

// ////////////////////////////////////////////////
// POLL //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddVoteResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddVote {
    pub poll_id: PollKey,
    pub accept: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreatePollReponse {
    pub id: PollKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreatePoll {
    pub voter_policy: VoterPolicy,
    pub visibility: PollVisibility,
    pub update_policy: PollPolicy,
    pub timeout_policy: PollPolicy,
    pub accept_payload: Box<Option<Action>>,
    pub reject_payload: Box<Option<Action>>,
    #[specta(type = Option<f64>)]
    pub duration: Option<Time>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct PollCleanupResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct PollCleanup {
    pub poll_id: PollKey,
    pub cancelled: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct PollTimeoutResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct PollTimeout {
    pub poll_id: PollKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveVoteResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoveVote {
    pub poll_id: PollKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdatePollsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdatePolls {}

// ////////////////////////////////////////////////
// PROSECUTION //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AdvanceProsecutionResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AdvanceProsecution {
    pub prosecution_id: ProsecutionKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CullProsecutionsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CullProsecutions {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ProsecutionVoteResResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct ProsecutionVoteRes {
    pub prosecution_id: ProsecutionKey,
    pub success: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SelectLawyerResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SelectLawyer {
    pub prosecution_id: ProsecutionKey,
    pub lawyer_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetCustodyResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetCustody {
    pub defendant_id: ActorKey,
    pub custody: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SignalReadyResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SignalReady {
    pub prosecution_id: ProsecutionKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct StartProsecutionResponse {
    pub id: ProsecutionKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct StartProsecution {
    pub source: ProsecutionSource,
    pub prosecutor_id: ActorKey,
    pub prosecutor_display: ActorDisplay,
    pub defendant_id: ActorKey,
    pub defendant_display: ActorDisplay,
    pub autonomous: bool,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct TerminateProsecutionResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct TerminateProsecution {
    pub prosecution_id: ProsecutionKey,
}

// ////////////////////////////////////////////////
// UPDATE //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct Update {}

// ////////////////////////////////////////////////
// WORLD //
// ////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddToWorldChannelsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct AddToWorldChannels {
    pub player_id: ActorKey,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateOrgsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateOrgs {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct InitializeEngineResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct InitializeEngine {
    pub seed: Seed,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct InitializeWorldResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct InitializeWorld {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetRandomSeedResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetRandomSeed {
    pub seed: Seed,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetWorldChannelOverrideResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct SetWorldChannelOverride {
    pub player_id: ActorKey,
    pub channel_name: WorldChannelName,
    pub source: OverrideSource,
    pub priority: u8,
    pub override_data: Option<WorldChannelOverride>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateWorldChannelPermsResponse {}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateWorldChannelPerms {
    pub player_id: ActorKey,
}

// ////////////////////////////////////////////////
// ACTION & RESPONSE ENUMS //
// ////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ActionContext {
    pub commands: Vec<CommandPayload>,
}

impl ActionContext {
    pub fn push_cmd(&mut self, cmd: Command, recipient: Option<ActorKey>, time: Time) {
        self.commands.push(CommandPayload {
            timestamp: time,
            recipient,
            cmd,
        });
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Type)]
pub enum Action {
    ChangeOrgLeader(ChangeOrgLeader),
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
    Null(Null),
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
    CullIncarcerations(CullIncarcerations),
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub enum ActionResponse {
    ChangeOrgLeader(ChangeOrgLeaderResponse),
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
    Null(NullResponse),
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
