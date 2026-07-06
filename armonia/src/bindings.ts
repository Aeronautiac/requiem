import { invoke } from "@tauri-apps/api/core";

// ////////////////////////////////////////////////////////////
// KEYS & PRIMITIVES
// ////////////////////////////////////////////////////////////

export type SlotKey = { idx: number; version: number };
export type ActorKey = SlotKey;
export type AbilityKey = SlotKey;
export type PassiveKey = SlotKey;
export type NotebookKey = SlotKey;
export type ChannelKey = SlotKey;
export type ChargePoolKey = SlotKey;
export type PollKey = SlotKey;
export type LoungeKey = SlotKey;
export type GroupchatKey = SlotKey;
export type BugKey = SlotKey;
export type ProsecutionKey = SlotKey;
export type KidnappingKey = SlotKey;
export type IncarcerationKey = SlotKey;

// ////////////////////////////////////////////////////////////
// SHARED ENUMS & STRUCTS
// ////////////////////////////////////////////////////////////

export type Role =
  | "Kira"
  | "SecondKira"
  | "L"
  | "Watari"
  | "BeyondBirthday"
  | "PrivateInvestigator"
  | "NewsAnchor"
  | "Civilian"
  | "RogueCivilian"
  | "Poser"
  | "ConArtist"
  | "WantedCivilian"
  | "Near"
  | "Mello";

export type AbilityName =
  | "Contact"
  | "CreateGroupChat"
  | "AnonymousContact"
  | "FalseAnonymousContact"
  | "AnonymousAnnouncement"
  | "FabricateLounge"
  | "Pseudocide"
  | "Bug"
  | "TapIn"
  | "Blackout"
  | "ShinigamiSacrifice"
  | "BackgroundCheck"
  | "CivilianArrest"
  | "UnlawfulArrest"
  | "UnderTheRadar"
  | "KiraConnection"
  | "AnonymousProsecute"
  | "Autopsy"
  | "Ipp"
  | "TrueNameReroll"
  | "PublicKidnap"
  | "AnonymousKidnap"
  | "TrueNameReveal"
  | "NotebookReveal"
  | "Gun"
  | "Prosecute"
  | "Outsource"
  | "TrueNameInvite"
  | "LeaderResign"
  | "ForceInvite"
  | "SilentProsecute";

export type ActorDisplay =
  | { Raw: ActorKey }
  | { Org: ActorKey }
  | { Role: Role }
  | "Mysterious"
  | "System";

// Individual State flag variant — used in AddState / RemoveState
export type State = "Dead" | "Incarcerated" | "Ipp" | "Kidnapped" | "Custody";
// BitFlags<State> — used in commands
export type States = number;
export const StateFlag = {
  Dead: 1 << 0,
  Incarcerated: 1 << 1,
  Ipp: 1 << 2,
  Kidnapped: 1 << 3,
  Custody: 1 << 4,
} as const;

// BitFlags<ChannelPermission>
export type ChannelPermissions = number;
export const ChannelPermissionFlag = {
  Send: 1 << 0,
  View: 1 << 1,
  LoggabilityControl: 1 << 2,
} as const;

export type ChannelMember = {
  perms: ChannelPermissions;
  displays: ActorDisplay[];
};

export type OrganizationName = "NULL" | "KK" | "TF" | "SPK";

// BitFlags<OrgAbilityPolicy>
export type OrgAbilityPolicies = number;
export const OrgAbilityPolicyFlag = {
  RequireLeader: 1 << 0,
  RequireVote: 1 << 1,
} as const;

export type OrgAbility = {
  require_roles: Role[];
  require_members: number;
  usage_policies: OrgAbilityPolicies;
};

// BitFlags<LeadershipTransferPolicy>
export type LeadershipTransferPolicies = number;
export const LeadershipTransferPolicyFlag = {
  Choose: 1 << 0,
  Random: 1 << 1,
} as const;

export type PoolLinkType = "Limit" | "Pool";

export type ContactLogType = "Full" | "Even" | "Odd";

export type PassiveType =
  | "Wanted"
  | { VoteAmplification: { multiplier: number } }
  | "VolatileEyes"
  | { ContactLogs: ContactLogType }
  | "OwnedNotebookBlock"
  | "CustodyBugReceiver";

export type VoterPolicy = "Present";

export type PollPolicy = "AlwaysInconclusive" | "Majority" | "WinningVote";

export type PollVisibility =
  | { Org: ActorKey }
  | { Channel: ChannelKey }
  | "AllPresent";

export type AnonymousLoungeRoleDisplay = "Dynamic" | { Static: Role };

export type LoungeVariant =
  | { Fake: { creator_id: ActorKey; contacted_id: ActorKey; contactor_id: ActorKey } }
  | { Basic: { contacted_id: ActorKey; contactor_id: ActorKey } }
  | { Anonymous: { contacted_id: ActorKey; contactor_id: ActorKey, role_display: AnonymousLoungeRoleDisplay } };

export type BugSource =
  | { Ability: AbilityKey }
  | "Custody";

export type ProsecutionSource =
  | "None"
  | { Ability: AbilityKey };

export type KidnappingType =
  | "Anonymous"
  | { Public: ActorDisplay };

export type KidnappingSource =
  | "None"
  | { Ability: AbilityKey };

export type IncarcerationSource =
  | "None"
  | { Ability: AbilityKey };

export type WorldChannelName = "News" | "General" | "Prison" | "LAndWatari";

export type WorldChannelOverride = {
  default_perms: ChannelPermissions;
  force_perms: ChannelPermissions;
};

export type OverrideSource =
  | { Role: Role }
  | { Manual: number }
  | { PressConference: ActorKey }
  | "Incarceration";

export type AbilityBehaviour =
  | { Contact: { target_id: ActorKey } }
  | { Pseudocide: { target_id: ActorKey; true_name: string; death_message: string; role: Role; notebook_transferred: boolean; ability_transferred: boolean } }
  | { Gun: { target_id: ActorKey } }
  | { AnonymousAnnouncement: { content: string } }
  | { AnonymousContact: { target: ActorKey } };

// ////////////////////////////////////////////////////////////
// ACTION STRUCTS
// ////////////////////////////////////////////////////////////

// -- ability --

export type AddAbility = {
  ability_name: AbilityName;
  transferrable: boolean;
  variant: number;
};

export type AddLink = {
  ability_id: AbilityKey;
  pool_id: ChargePoolKey;
  weight: number;
  link_type: PoolLinkType;
  volatile: boolean;
};

export type ClearLinks = {
  ability_id: AbilityKey;
};

export type ClearVolatileLinks = {
  ability_id: AbilityKey;
};

export type CreateAndGiveAbility = {
  ability_name: AbilityName;
  transferrable: boolean;
  variant: number;
  actor_id: ActorKey;
  volatile: boolean;
};

export type DestroyAbility = {
  ability_id: AbilityKey;
};

export type GiveAbility = {
  ability_id: AbilityKey;
  actor_id: ActorKey;
  volatile: boolean;
};

export type RemoveLink = {
  ability_id: AbilityKey;
  pool_id: ChargePoolKey;
};

export type UseAbility = {
  ability_id: AbilityKey;
  ability_args: AbilityBehaviour;
};

// -- actor --

export type AddState = {
  actor_id: ActorKey;
  state: State;
};

export type CreateActorLinks = Record<string, never>;

export type PurgeVolatiles = {
  actor_id: ActorKey;
};

export type RemoveState = {
  actor_id: ActorKey;
  state: State;
};

export type SeverLinks = {
  actor_id: ActorKey;
};

// -- org --

export type AddToOrg = {
  leader: boolean;
  og: boolean;
  actor_id: ActorKey;
  org_id: ActorKey;
};

export type ChangeOrgLeader = {
  org_id: ActorKey;
  new_leader: ActorKey | null;
};

export type CreateAndGiveOrgAbility = {
  ability_name: AbilityName;
  variant: number;
  org_id: ActorKey;
  settings: OrgAbility;
};

export type CreateOrg = {
  name: OrganizationName;
};

export type GiveOrgAbility = {
  org_id: ActorKey;
  ability_id: AbilityKey;
  settings: OrgAbility;
};

export type RemoveFromOrg = {
  actor_id: ActorKey;
  org_id: ActorKey;
};

export type SetLeadership = {
  org_id: ActorKey;
  policies: LeadershipTransferPolicies | null;
};

export type SystemUseOrgAbility = {
  org_id: ActorKey;
  user_id: ActorKey;
  ability_id: AbilityKey;
  ability_args: AbilityBehaviour;
  dont_vote: boolean;
};

export type UseOrgAbility = {
  org_id: ActorKey;
  ability_id: AbilityKey;
  ability_args: AbilityBehaviour;
};

// -- player --

export type AddPlayer = {
  true_name: string;
  starting_role: Role;
};

export type GiveRole = {
  target_id: ActorKey;
  role: Role;
};

export type Kill = {
  target_id: ActorKey;
  killer_id: ActorKey | null;
  death_message: string | null;
  silent: boolean;
  allow_link_chaining: boolean;
  sever_links: boolean;
  set_books_dormant: boolean;
};

export type Revive = {
  ignore_links: boolean;
  target_id: ActorKey;
};

export type ScheduleKill = {
  timestamp: number;
  kill: Kill;
  notebook_scheduled: boolean;
};

export type ScheduleRevive = {
  timestamp: number;
  revive: Revive;
};

// -- chargepool --

export type AddChargePool = {
  base_charges: number;
  base_reset_time: number;
};

export type AddCharges = {
  id: ChargePoolKey;
  charges: number;
};

export type TryDeleteChargePool = {
  id: ChargePoolKey;
};

// -- bug --

export type ArchiveBug = {
  bug_id: BugKey;
};

export type CreateBug = {
  target_id: ActorKey;
  source: BugSource;
};

export type DestroyBug = {
  bug_id: BugKey;
};

export type UpdateBugVisibilities = Record<string, never>;

// -- channel --

export type CreateChannel = {
  loggable: boolean;
};

export type DestroyChannel = {
  channel_id: ChannelKey;
  archive: boolean;
};

export type SendMessage = {
  channel_id: ChannelKey;
  display: ActorDisplay;
  content: string;
};

export type SetLoggable = {
  channel_id: ChannelKey;
  loggable: boolean;
};

export type SetMember = {
  player_id: ActorKey;
  channel_id: ChannelKey;
  settings: ChannelMember | null;
};

// -- groupchat --

export type AddToGroupchat = {
  groupchat_id: GroupchatKey;
  player_id: ActorKey;
  owner: boolean;
};

export type CreateGroupchat = Record<string, never>;

export type RemoveFromGroupchat = {
  groupchat_id: GroupchatKey;
  player_id: ActorKey;
};

export type SetGroupchatOwner = {
  groupchat_id: GroupchatKey;
  owner: ActorKey | null;
};

// -- lounge --

export type CreateLounge = {
  variant: LoungeVariant;
};

export type LeaveLounge = {
  lounge_id: LoungeKey;
};

export type RemoveFromLounge = {
  lounge_id: LoungeKey;
  player_id: ActorKey;
};

export type UpdateContactChannels = {
  player_id: ActorKey;
};

// -- engine --

export type Null = Record<string, never>;

export type ScheduleJob = {
  timestamp: number;
  payload: Action;
};

// -- incarceration --

export type CreateIncarceration = {
  victim_id: ActorKey;
  source: IncarcerationSource;
};

export type CullIncarcerations = {
  ability_id: AbilityKey;
};

export type ReleaseIncarceration = {
  incarceration_id: IncarcerationKey;
  forced: boolean;
};

export type UpdatePrisonChannel = {
  actor_id: ActorKey;
};

// -- kidnapping --

export type CreateKidnapping = {
  victim_id: ActorKey;
  kidnapping_type: KidnappingType;
  source: KidnappingSource;
};

export type CullKidnappings = {
  ability_id: AbilityKey;
};

export type ReleaseKidnapping = {
  kidnapping_id: KidnappingKey;
  forced: boolean;
};

export type UpdateKidnapChannels = Record<string, never>;

// -- notebook --

export type AddNotebook = {
  fake: boolean;
};

export type CreateAndGiveNotebook = {
  fake: boolean;
  actor_id: ActorKey;
  volatile: boolean;
};

export type DestroyNotebook = {
  notebook_id: NotebookKey;
};

export type GiveNotebook = {
  notebook_id: NotebookKey;
  actor_id: ActorKey;
  volatile: boolean;
};

export type LendNotebook = {
  notebook_id: NotebookKey;
  target_id: ActorKey;
};

export type NotebookScheduledKill = {
  kill: Kill;
};

export type ReturnDormantBooks = {
  actor_id: ActorKey;
};

export type SetBooksDormant = {
  actor_id: ActorKey;
};

export type SetBorrowersToOwners = {
  actor_id: ActorKey;
};

export type TakeNotebook = {
  notebook_id: NotebookKey;
};

export type WriteName = {
  true_name: string;
  death_message: string | null;
  notebook_id: NotebookKey;
  delay: number;
};

// -- passive --

export type AddPassive = {
  passive_type: PassiveType;
  transferrable: boolean;
};

export type CreateAndGivePassive = {
  passive_type: PassiveType;
  transferrable: boolean;
  actor_id: ActorKey;
  volatile: boolean;
};

export type DestroyPassive = {
  passive_id: PassiveKey;
};

export type GivePassive = {
  passive_id: PassiveKey;
  actor_id: ActorKey;
  volatile: boolean;
};

// -- poll --

export type AddVote = {
  poll_id: PollKey;
  accept: boolean;
};

export type CreatePoll = {
  voter_policy: VoterPolicy;
  visibility: PollVisibility;
  update_policy: PollPolicy;
  timeout_policy: PollPolicy;
  accept_payload: Action | null;
  reject_payload: Action | null;
  duration: number | null;
};

export type PollCleanup = {
  poll_id: PollKey;
  cancelled: boolean;
};

export type PollTimeout = {
  poll_id: PollKey;
};

export type RemoveVote = {
  poll_id: PollKey;
};

export type UpdatePolls = Record<string, never>;

// -- prosecution --

export type AdvanceProsecution = {
  prosecution_id: ProsecutionKey;
};

export type CullProsecutions = Record<string, never>;

export type ProsecutionVoteRes = {
  prosecution_id: ProsecutionKey;
  success: boolean;
};

export type SelectLawyer = {
  prosecution_id: ProsecutionKey;
  lawyer_id: ActorKey;
};

export type SetCustody = {
  defendant_id: ActorKey;
  custody: boolean;
};

export type SignalReady = {
  prosecution_id: ProsecutionKey;
};

export type StartProsecution = {
  source: ProsecutionSource;
  prosecutor_id: ActorKey;
  prosecutor_display: ActorDisplay;
  defendant_id: ActorKey;
  defendant_display: ActorDisplay;
  autonomous: boolean;
};

export type TerminateProsecution = {
  prosecution_id: ProsecutionKey;
};

// -- update --

export type Update = Record<string, never>;

// -- world --

export type AddToWorldChannels = {
  player_id: ActorKey;
};

export type CreateOrgs = Record<string, never>;

export type InitializeEngine = {
  seed: number;
};

export type InitializeWorld = Record<string, never>;

export type SetRandomSeed = {
  seed: number;
};

export type SetWorldChannelOverride = {
  player_id: ActorKey;
  channel_name: WorldChannelName;
  source: OverrideSource;
  priority: number;
  override_data: WorldChannelOverride | null;
};

export type UpdateWorldChannelPerms = {
  player_id: ActorKey;
};

// ////////////////////////////////////////////////////////////
// ACTION & REQUEST
// ////////////////////////////////////////////////////////////

export type Action =
  | { ChangeOrgLeader: ChangeOrgLeader }
  | { Kill: Kill }
  | { AddState: AddState }
  | { Revive: Revive }
  | { AddPlayer: AddPlayer }
  | { AddNotebook: AddNotebook }
  | { GiveNotebook: GiveNotebook }
  | { WriteName: WriteName }
  | { LendNotebook: LendNotebook }
  | { ScheduleKill: ScheduleKill }
  | { RemoveState: RemoveState }
  | { GiveRole: GiveRole }
  | { AddAbility: AddAbility }
  | { DestroyAbility: DestroyAbility }
  | { UseAbility: UseAbility }
  | { ScheduleRevive: ScheduleRevive }
  | { GiveAbility: GiveAbility }
  | { AddPassive: AddPassive }
  | { DestroyPassive: DestroyPassive }
  | { GivePassive: GivePassive }
  | { SeverLinks: SeverLinks }
  | { CreateActorLinks: CreateActorLinks }
  | { PurgeVolatiles: PurgeVolatiles }
  | { CreateAndGiveAbility: CreateAndGiveAbility }
  | { CreateAndGiveNotebook: CreateAndGiveNotebook }
  | { DestroyNotebook: DestroyNotebook }
  | { CreateAndGivePassive: CreateAndGivePassive }
  | { TakeNotebook: TakeNotebook }
  | { Null: Null }
  | { SetBorrowersToOwners: SetBorrowersToOwners }
  | { SetBooksDormant: SetBooksDormant }
  | { ReturnDormantBooks: ReturnDormantBooks }
  | { NotebookScheduledKill: NotebookScheduledKill }
  | { TryDeleteChargePool: TryDeleteChargePool }
  | { InitializeWorld: InitializeWorld }
  | { AddChargePool: AddChargePool }
  | { ClearVolatileLinks: ClearVolatileLinks }
  | { UseOrgAbility: UseOrgAbility }
  | { Update: Update }
  | { UpdatePolls: UpdatePolls }
  | { CreatePoll: CreatePoll }
  | { PollTimeout: PollTimeout }
  | { ScheduleJob: ScheduleJob }
  | { AddVote: AddVote }
  | { RemoveVote: RemoveVote }
  | { PollCleanup: PollCleanup }
  | { AddToOrg: AddToOrg }
  | { RemoveFromOrg: RemoveFromOrg }
  | { CreateOrg: CreateOrg }
  | { SystemUseOrgAbility: SystemUseOrgAbility }
  | { AddCharges: AddCharges }
  | { AddLink: AddLink }
  | { RemoveLink: RemoveLink }
  | { ClearLinks: ClearLinks }
  | { CreateOrgs: CreateOrgs }
  | { SetLeadership: SetLeadership }
  | { GiveOrgAbility: GiveOrgAbility }
  | { CreateAndGiveOrgAbility: CreateAndGiveOrgAbility }
  | { SendMessage: SendMessage }
  | { CreateChannel: CreateChannel }
  | { DestroyChannel: DestroyChannel }
  | { SetMember: SetMember }
  | { SetLoggable: SetLoggable }
  | { CreateLounge: CreateLounge }
  | { UpdateContactChannels: UpdateContactChannels }
  | { LeaveLounge: LeaveLounge }
  | { RemoveFromLounge: RemoveFromLounge }
  | { AddToGroupchat: AddToGroupchat }
  | { CreateGroupchat: CreateGroupchat }
  | { SetGroupchatOwner: SetGroupchatOwner }
  | { RemoveFromGroupchat: RemoveFromGroupchat }
  | { CreateBug: CreateBug }
  | { ArchiveBug: ArchiveBug }
  | { DestroyBug: DestroyBug }
  | { StartProsecution: StartProsecution }
  | { SetCustody: SetCustody }
  | { AdvanceProsecution: AdvanceProsecution }
  | { SignalReady: SignalReady }
  | { SelectLawyer: SelectLawyer }
  | { CullProsecutions: CullProsecutions }
  | { TerminateProsecution: TerminateProsecution }
  | { AddToWorldChannels: AddToWorldChannels }
  | { UpdateWorldChannelPerms: UpdateWorldChannelPerms }
  | { SetWorldChannelOverride: SetWorldChannelOverride }
  | { InitializeEngine: InitializeEngine }
  | { SetRandomSeed: SetRandomSeed }
  | { UpdateBugVisibilities: UpdateBugVisibilities }
  | { ProsecutionVoteRes: ProsecutionVoteRes }
  | { CreateKidnapping: CreateKidnapping }
  | { ReleaseKidnapping: ReleaseKidnapping }
  | { CullKidnappings: CullKidnappings }
  | { UpdateKidnapChannels: UpdateKidnapChannels }
  | { UpdatePrisonChannel: UpdatePrisonChannel }
  | { CreateIncarceration: CreateIncarceration }
  | { ReleaseIncarceration: ReleaseIncarceration }
  | { CullIncarcerations: CullIncarcerations };

export type OrgActorInfo = {
  org_id: ActorKey;
  player_id: ActorKey;
};

export type ActionActor =
  | "Admin"
  | "System"
  | { Player: ActorKey }
  | { Organization: OrgActorInfo };

export type ActionRequest = {
  actor: ActionActor;
  timestamp: number;
  payload: Action;
};

// ////////////////////////////////////////////////////////////
// ACTION RESPONSE
// ////////////////////////////////////////////////////////////

export type ActionError =
  | "ActorNotFound"
  | "ActorIsDead"
  | "ActorIsAlive"
  | "ActorHasNotebookReceiveRestriction"
  | "InsufficientPermissions"
  | "ActorIsNotPlayer"
  | "NameNotUnique"
  | "NotebookNotFound"
  | "NotebookNotOwned"
  | "NotebookUsageBlocked"
  | "NotebookPassageBlocked"
  | "NotebookOnCooldown"
  | "CannotLendToYourself"
  | "TimeAlreadyPassed"
  | "AbilityCategoryBlocked"
  | "NotEnoughMembers"
  | "RequiredRolesNotPresent"
  | "PassiveNotFound"
  | "AbilityConfigNotFound"
  | "AbilityNotFound"
  | "ActorIsSystem"
  | "AbilityNotOwned"
  | "AbilityMismatch"
  | "AbilityNotEnoughCharges"
  | "RoleNotImplemented"
  | "ItemAlreadyOwned"
  | "ItemAlreadyUnowned"
  | "ChargePoolNotFound"
  | "ActorIsNotOrg"
  | "PlayerIsNotLeader"
  | "PollDoesntExist"
  | "InvalidVoter"
  | "NotAVoter"
  | "AlreadyVoted"
  | "PlayerIsBlacklisted"
  | "OrgDoesntHaveLeadership"
  | "ActorAlreadyInOrg"
  | "UserNotPresent"
  | "PlayerNotInOrg"
  | "AlreadyLeader"
  | "ChannelDoesntExist"
  | "NotAChannelMember"
  | "DisplayNotOwned"
  | "PlayerNotInLounge"
  | "LoungeDoesntExist"
  | "GroupchatDoesntExist"
  | "CannotContact"
  | "CannotContactSelf"
  | "NotTheOwner"
  | "NotInGroupchat"
  | "BugNotFound"
  | "ProsecutionNotFound"
  | "AlreadyADefendant"
  | "NotInProsecution"
  | "NotACustodyPhase"
  | "IncompatiblePhase"
  | "AlreadySignalled"
  | "LawyerAlreadySelected"
  | "CannotBeOwnLawyer"
  | "KidnappingNotFound"
  | "IncarcerationNotFound"
  | "ActorHasStrengthenedPresence";

// Only variants that carry meaningful data are included.
export type ActionResponse =
  | { AddPlayer: { id: ActorKey } }
  | { AddNotebook: { id: NotebookKey } }
  | { AddAbility: { id: AbilityKey } }
  | { AddPassive: { id: PassiveKey } }
  | { CreateAndGiveAbility: { id: AbilityKey } }
  | { CreateAndGiveNotebook: { id: NotebookKey } }
  | { CreateAndGivePassive: { id: PassiveKey } }
  | { AddChargePool: { id: ChargePoolKey } }
  | { UseOrgAbility: { poll_id: PollKey | null } }
  | { CreatePoll: { id: PollKey } }
  | { CreateOrg: { id: ActorKey } }
  | { SystemUseOrgAbility: { poll_id: PollKey | null } }
  | { CreateAndGiveOrgAbility: { id: AbilityKey } }
  | { CreateChannel: { id: ChannelKey } }
  | { CreateLounge: { lounge_id: LoungeKey; channel_id: ChannelKey } }
  | { CreateGroupchat: { id: GroupchatKey } }
  | { CreateBug: { id: BugKey } }
  | { StartProsecution: { id: ProsecutionKey } }
  | { CreateKidnapping: { id: KidnappingKey } }
  | { CreateIncarceration: { id: IncarcerationKey } };

// ////////////////////////////////////////////////////////////
// COMMANDS (frontend instructions inside ActionContext)
// ////////////////////////////////////////////////////////////

export type Command =
  | { Death: { true_name: string; death_message: string; role: Role; notebook_transferred: boolean; ability_transferred: boolean } }
  | { Kidnapping: { target_id: ActorKey; duration: number } }
  | { KidnapReveal: Record<string, never> }
  | { PseudocideRevival: { target_id: ActorKey } }
  | { AnonymousAnnouncement: { content: string } }
  | { MapOrg: { org_id: ActorKey; actor_id: ActorKey } }
  | { ActorState: { state: States; actor_id: ActorKey } }
  | { AddOrgMember: { player_id: ActorKey; org_id: ActorKey } }
  | { RemoveOrgMember: { player_id: ActorKey; org_id: ActorKey } }
  | { AddMessage: { content: string; channel_id: ChannelKey; sender_display: ActorDisplay } }
  | { MapLounge: { lounge_id: LoungeKey; channel_id: ChannelKey } }
  | { MapGc: { gc_id: GroupchatKey; channel_id: ChannelKey } }
  | { MapWorldChannel: { channel_name: WorldChannelName; channel_id: ChannelKey } }
  | { DeleteChannel: { channel_id: ChannelKey } }
  | { ArchiveChannel: { channel_id: ChannelKey } }
  | { NewBug: { bug_key: BugKey } }
  | { AddBugMessage: { bug_key: BugKey; display: ActorDisplay; content: string } }
  | { ArchiveBug: { bug_key: BugKey } }
  | { ClearBugVisibily: { bug_id: BugKey } }
  | { DeleteBug: { bug_id: BugKey } }
  | { RemoveChannel: { channel_id: ChannelKey } }
  | { GcOwnerStatus: { owner: boolean; gc_id: GroupchatKey } }
  | { ShowChannelMember: { channel_id: ChannelKey; display: ActorDisplay; channel_perms: ChannelPermissions } }
  | { RemoveChannelMember: { channel_id: ChannelKey; display: ActorDisplay } }
  | { UpdateChannelView: { channel_id: ChannelKey; perms: ChannelPermissions; displays: ActorDisplay[] } }
  | { SetBugVisibility: { bug_id: BugKey; visible: boolean } }
  | { MapNotebook: { notebook_id: NotebookKey; channel_id: ChannelKey } }
  | { NotebookWrite: { notebook_id: NotebookKey; user_id: ActorKey; message: string | null; true_name: string; delay: number; successes_remaining: number; attempts_remaining: number; success: boolean; target_saved: boolean } }
  | { NotebookBorrowingStatus: { borrowed: boolean } }
  | { AddContactLog: { passive_id: PassiveKey } }
  | { UpdateAbilityView: { ability_name: AbilityName; usages_remaining: number; iterations_to_reset: number; ability_id: AbilityKey; owner_id: ActorKey } }
  | { RemoveAbility: { ability_id: AbilityKey } }
  | { RevealAutopsyMessages: { target_id: ActorKey; range: number; redact_names: boolean } };

export type CommandRecipient = "System" | "BasePlayer" | {
  Player: ActorKey
}

export type CommandPayload = {
  timestamp: number;
  recipient: CommandRecipient;
  cmd: Command;
};

export type ActionContext = {
  commands: CommandPayload[];
};

// ////////////////////////////////////////////////////////////
// IPC ENVELOPE
// ////////////////////////////////////////////////////////////

export type IpcExecutionResult =
  | { Ok: [ActionResponse, ActionContext] }
  | { Err: ActionError };

export type AppExecResult =
  | { Standard: IpcExecutionResult }
  | "Crashed";

export type AppExecution = {
  exec_result: AppExecResult;
};

// ////////////////////////////////////////////////////////////
// KEY HELPERS
// ////////////////////////////////////////////////////////////

export function slotKeyToString(key: SlotKey): string {
  return `${key.idx}:${key.version}`;
}

export function slotKeyFromString(s: string): SlotKey {
  const [idx, version] = s.split(":").map(Number);
  return { idx, version };
}

// ////////////////////////////////////////////////////////////
// BITFLAG HELPERS
// ////////////////////////////////////////////////////////////

export function hasFlag(bitfield: number, flag: number): boolean {
  return (bitfield & flag) !== 0;
}

export function addFlag(bitfield: number, flag: number): number {
  return bitfield | flag;
}

export function removeFlag(bitfield: number, flag: number): number {
  return bitfield & ~flag;
}

export function combineFlags(...values: number[]): number {
  return values.reduce((acc, f) => acc | f, 0);
}

// ////////////////////////////////////////////////////////////
// TAURI COMMAND
// ////////////////////////////////////////////////////////////

export function sendAction(action: ActionRequest): Promise<AppExecution> {
  return invoke("send_action", { action });
}
