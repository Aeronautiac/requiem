// The wire contract, hand-written (no codegen) and shared by every host. Nothing in this
// file may import from a platform: a host that cannot run Tauri, or cannot run in a
// browser, still has to be able to depend on these types.

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
export type ViewportKey = SlotKey;

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
  | "CreateGroupchat"
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

export type PoolLinkType = "Restrictive" | "Permissive";

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

// What a poll is about. The org/channel is carried by the poll's scope (visibility), so
// subjects never repeat it. Generic is the pre-rendered fallback.
export type PollSubject =
  | { OrgAbility: AbilityBehaviour }
  | { CivilianArrest: ActorKey }
  | { Generic: string };

export type PollOutcome = "Accepted" | "Rejected" | "Inconclusive" | "Cancelled";

export type AnonymousLoungeRoleDisplay = "Dynamic" | { Static: Role };

export type LoungeVariant =
  | { Fake: { creator_id: ActorKey; contacted_id: ActorKey; contactor_id: ActorKey } }
  | { Basic: { contacted_id: ActorKey; contactor_id: ActorKey } }
  | { Anonymous: { contacted_id: ActorKey; contactor_id: ActorKey, role_display: AnonymousLoungeRoleDisplay } };

export type BugSource =
  | { Ability: AbilityKey }
  | "Custody";

// Target-facing bug context (see the engine's BugContext): why a bugged player is under
// surveillance, with the owner deliberately stripped out.
export type BugContext = "Explicit" | "Custody";

export type ProsecutionSource =
  | "None"
  | { Ability: AbilityKey };

// whether the side holding the floor has started. In Grace their first message starts their slot;
// in Presentation the clock is already running on it.
export type TrialSubphaseView = "Grace" | "Presentation";

// which side holds the floor during the trial phase
export type TrialPhaseView =
  | { Prosecutor: TrialSubphaseView }
  | { Defense: TrialSubphaseView }
  | { Debate: { prosecutor_done: boolean; defense_done: boolean } };

// client-facing snapshot of where a prosecution is in its lifecycle. The ready/done flags sit
// inside the phase that owns them, so a phase without them cannot be described as having them.
export type ProsecutionPhaseView =
  | { Custody: { prosecutor_ready: boolean; defense_ready: boolean } }
  | { Trial: TrialPhaseView }
  | "Voting";

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
  | { AnonymousContact: { target: ActorKey } }
  | { CreateGroupchat: Record<string, never> }
  | { FabricateLounge: { contacted_id: ActorKey; contactor_id: ActorKey } }
  | { FalseAnonymousContact: { target: ActorKey; role: Role } }
  | { Ipp: { target: ActorKey } }
  | { Prosecute: { target: ActorKey } }
  | { AnonymousProsecute: { target: ActorKey } }
  | { TrueNameInvite: { target: ActorKey; true_name: string } }
  | { ForceInvite: { target: ActorKey } }
  | { BackgroundCheck: { target: ActorKey } }
  | { Outsource: { invitee: ActorKey; defendant: ActorKey } }
  | { LeaderResign: { successor: ActorKey | null } }
  | { TrueNameReveal: { target: ActorKey } }
  | { NotebookReveal: { target: ActorKey } }
  | { CivilianArrest: { target: ActorKey } }
  | { PublicKidnap: { target: ActorKey; performer: ActorKey | null } }
  | { AnonymousKidnap: { target: ActorKey } }
  | { Bug: { target: ActorKey } };

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

// tearing a channel down is always archival, so there is no flag to select between.
export type DestroyChannel = {
  channel_id: ChannelKey;
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

export type SetTrueName = {
  target_id: ActorKey;
  true_name: string;
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

// Player action: create a personal channel (a private notepad / line to whoever bugged you).
// Takes no args — the acting player is the owner.
export type CreatePersonalChannel = Record<string, never>;

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
export type Crash = Record<string, never>;

export type ScheduleJob = {
  timestamp: number;
  payload: Action;
};

// -- incarceration --

export type CreateIncarceration = {
  victim_id: ActorKey;
  source: IncarcerationSource;
  // null = held until someone releases them; a value schedules the release.
  duration: number | null;
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
  subject: PollSubject;
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
  | { Crash: Crash }
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
  | { CreatePersonalChannel: CreatePersonalChannel }
  | { DestroyChannel: DestroyChannel }
  | { SetMember: SetMember }
  | { SetLoggable: SetLoggable }
  | { SetTrueName: SetTrueName }
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
  | "EngineAlreadyInitialized"
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
  | "ActorHasStrengthenedPresence"
  | "PersonalChannelLimitReached"
  | "PerformerRequiresOrg";

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
  // access changes, addressed to the actor whose access changed. gaining access delivers
  // everything previously addressed to that viewport, in order; losing it only stops further
  // delivery, and never retracts anything already received. `kind` is a display aid — nothing
  // here or on the server may branch on it.
  | { EnterViewport: { viewport: ViewportKey; actor: ActorKey; kind: ViewportKind } }
  | { ExitViewport: { viewport: ViewportKey; actor: ActorKey } }
  | { Death: { target_id: ActorKey, true_name: string; death_message: string; role: Role; notebook_transferred: boolean; ability_transferred: boolean } }
  | { Kidnapping: { kidnapping_id: KidnappingKey; target_id: ActorKey; duration: number | null } }
  | { KidnapReveal: { kidnapping_id: KidnappingKey; kidnapper: ActorKey | null } }
  // Mirrors Kidnapping, minus any reveal — an incarceration's source is never disclosed.
  | { Incarceration: { incarceration_id: IncarcerationKey; victim_id: ActorKey; duration: number | null } }
  | { IncarcerationReleased: { incarceration_id: IncarcerationKey } }
  | { PseudocideRevival: { target_id: ActorKey } }
  | { AnonymousAnnouncement: { content: string } }
  | { MapOrg: { org_id: ActorKey; channel_id: ChannelKey; org_name: OrganizationName } }
  // directed to the actor itself: your own states. there is no broadcast form — what OTHER
  // viewers may know about an actor is announced by the event that caused it (Death, Kidnapping,
  // Bugged, …), and other actors' status is rendered from those.
  | { ActorState: { state: States; actor_id: ActorKey } }
  | { AddOrgMember: { player_id: ActorKey; org_id: ActorKey } }
  | { RemoveOrgMember: { player_id: ActorKey; org_id: ActorKey } }
  | { AddMessage: { content: string; channel_id: ChannelKey; sender_display: ActorDisplay } }
  // A player slot exists. No presentation rides here — a display name is a SERVER-level fact about
  // who is on the slot and arrives on its own channel (see ProfileUpdate); an unnamed slot falls
  // back to a generated "player-<idx>v<version>" label, like the other unnamed objects.
  // Addressed to the presence viewport, so entry backfills the whole roster.
  | { MapPlayer: { player_id: ActorKey } }
  | { MapLounge: { lounge_id: LoungeKey; channel_id: ChannelKey; contact_id: number } }
  | { MapGc: { gc_id: GroupchatKey; channel_id: ChannelKey; contact_id: number } }
  | { MapWorldChannel: { channel_name: WorldChannelName; channel_id: ChannelKey } }
  | { MapPersonalChannel: { channel_id: ChannelKey } }
  // the channel is finished; no further content will ever be addressed there. absorbed the old
  // DeleteChannel — nothing said in it can be un-said, so archival is all there is to express.
  | { ArchiveChannel: { channel_id: ChannelKey } }
  | { SetChannelLoggable: { channel_id: ChannelKey; loggable: boolean } }
  | { NewBug: { bug_key: BugKey } }
  | { AddBugMessage: { bug_key: BugKey; display: ActorDisplay; content: string } }
  // the bug is no longer active. absorbed the old DeleteBug for the same reason as ArchiveChannel.
  | { ArchiveBug: { bug_key: BugKey } }
  // directed to the bug's target: you're under surveillance, and in what context (never by whom)
  | { Bugged: { context: BugContext } }
  | { GcOwnerStatus: { owner: boolean; gc_id: GroupchatKey } }
  | { ShowChannelMember: { channel_id: ChannelKey; display: ActorDisplay; channel_perms: ChannelPermissions } }
  | { RemoveChannelMember: { channel_id: ChannelKey; display: ActorDisplay } }
  | { UpdateChannelView: { channel_id: ChannelKey; perms: ChannelPermissions; displays: ActorDisplay[] } }
  | { MapNotebook: { notebook_id: NotebookKey; channel_id: ChannelKey } }
  | { NotebookWrite: { notebook_id: NotebookKey; user_id: ActorKey; message: string | null; true_name: string; delay: number; successes_remaining: number; attempts_remaining: number; success: boolean; target_saved: boolean } }
  | { NotebookBorrowingStatus: { notebook_id: NotebookKey; borrowed: boolean } }
  | { AddContactLog: { passive_id: PassiveKey } }
  | { UpdateAbilityView: { ability_name: AbilityName; success_usages_remaining: number; failure_usages_remaining: number; iterations_to_reset: number; ability_id: AbilityKey; owner_id: ActorKey } }
  | { RemoveAbility: { ability_id: AbilityKey } }
  | { UpdatePassiveView: { passive_type: PassiveType; passive_id: PassiveKey; owner_id: ActorKey } }
  | { RemovePassive: { passive_id: PassiveKey } }
  | { RevealAutopsyMessages: { target_id: ActorKey; range: number; redact_names: boolean } }
  | { RevealTrueName: { target_id: ActorKey; true_name: string } }
  | { RevealNotebookHolding: { target_id: ActorKey; holding: boolean } }
  | { RoleUpdate: { target_id: ActorKey; role: Role } }
  | { TrueNameUpdate: { target_id: ActorKey; true_name: string } }
  | { UpdatePoll: { poll_id: PollKey; subject: PollSubject; scope: PollVisibility; accept: number; reject: number; potential: number; opener: ActorKey | null } }
  // the poll concluded. it closes, it does not disappear: whoever could see it keeps it and its
  // outcome.
  | { ClosePoll: { poll_id: PollKey; outcome: PollOutcome } }
  | { UpdatePollView: { poll_id: PollKey; eligible: boolean; own_vote: boolean | null } }
  // prosecutions: addressed to the presence viewport, plus a System mirror. the ordered timeline
  // is preserved by that viewport — an absent player exits it and re-entry replays every update
  // they missed, in order. trial_channel tags a channel as a prosecution channel so it renders
  // differently; the channel/poll contents ride their own viewports.
  | { UpdateProsecution: { prosecution_id: ProsecutionKey; prosecutor_display: ActorDisplay; defendant_display: ActorDisplay; phase: ProsecutionPhaseView; trial_channel: ChannelKey | null; lawyer_display: ActorDisplay | null } }
  // The private channel a defendant shares with their chosen lawyer, addressed to its own viewport
  // so only those two learn it exists.
  | { MapLawyerChannel: { channel_id: ChannelKey; prosecution_id: ProsecutionKey } }
  | { CloseProsecution: { prosecution_id: ProsecutionKey } };

// what kind of object a viewport belongs to. display only — do not branch on it.
export type ViewportKind = "Channel" | "Bug" | "Poll" | "Passive" | "Presence";

// every command is addressed. there is no "no recipient" case: what looks undirected is addressed
// to some object's viewport, and the object decides who may read it.
export type CommandRecipient = "System" | { Actor: ActorKey } | { Viewport: ViewportKey };

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
//
// The direct-IPC shape, used ONLY by a host running an engine in-process (armonia). It is
// the engine's ExecutionResult verbatim. The server envelope below is what the protocol is
// really defined in terms of; armonia's transport adapts this into that.

export type IpcExecutionResult =
  | { Ok: [ActionResponse, ActionContext] }
  | { Err: [ActionError, ActionContext] };

export type AppExecResult =
  | { Standard: IpcExecutionResult }
  | "Crashed";

export type AppExecution = {
  exec_result: AppExecResult;
};

// ////////////////////////////////////////////////////////////
// SERVER ENVELOPE (yagami)
// ////////////////////////////////////////////////////////////
//
// Hand-written to match yagami/src/main.rs, in serde's default externally-tagged form:
// unit variants are bare strings, everything else a single-key object. Kept in step by
// hand — there is no codegen on this boundary.

// What a client may send. `Action` reaches the engine; `Control` never does — controls act
// ON the game (teardown, key management) rather than in the fiction.
export type ServerInput =
  | { Action: ActionRequest }
  | { Control: GameControl };

export type Capability = "Administer" | "Supervise";

// Which actors a key may act as / observe. `All` also covers actors created later.
export type ActorScope = "All" | { Only: ActorKey[] };

export type GameControl =
  | "EndGame"
  | { CreateKey: { actors: ActorScope; capabilities: Capability[] } }
  | { RevokeKey: { key: string } }
  | { SetCapabilities: { key: string; capabilities: Capability[] } }
  | { SetActorScope: { key: string; actors: ActorScope } }
  // State what the server knows about whoever is playing an actor slot. Separate from creating
  // the slot: the engine's AddPlayer knows nothing about presentation, a slot may exist unnamed,
  // and a profile can change later without the engine ever hearing about it.
  //
  // REPLACES, like the two above — one control for the whole profile rather than one per field,
  // since every part of a profile has identical semantics. Adding a field adds no control.
  | { SetProfile: { actor: ActorKey; profile: Profile } };

export type ControlResponse =
  | "Ended"
  | "KeyRevoked"
  | "CapabilitiesSet"
  | "ActorScopeSet"
  | "ProfileSet"
  | { KeyCreated: { key: string } };

// A control refused on its own terms — the caller IS an administrator, just not over this
// target. Distinct from "Denied", which means not an administrator at all.
export type ControlError =
  | "KeyNotFound"
  | "CannotActOnSelf"
  | "RequiresSupervise"
  | "CannotGrantSupervise";

// Split by what was asked, then by how it went. "Denied" means this connection's key did
// not permit it; "Crashed" means the engine died holding the action.
export type ActionOutcome =
  | { Ok: ActionResponse }
  | { Err: ActionError }
  | "Denied"
  | "Crashed";

export type ControlOutcome =
  | { Ok: ControlResponse }
  | { Err: ControlError }
  | "Denied";

export type ExecOutcome =
  | { Action: ActionOutcome }
  | { Control: ControlOutcome };

// The reply echoes the input it answers, which is what lets a client match a reply to what
// it sent without the protocol carrying a correlation id.
export type ResponsePair = {
  input: ServerInput;
  output: ExecOutcome;
};

// One atomic ordered unit. `commands` is ALREADY filtered to what this connection may see —
// the server decides that, not the client. `response` is set only on the copy going to the
// connection that submitted the input, and null on everyone else's.
export type Batch = {
  commands: CommandPayload[];
  response: ResponsePair | null;
};

// What the SERVER knows about whoever occupies an actor slot — as opposed to MapPlayer, which is
// the ENGINE saying the slot exists at all. Two facts with different lifetimes: a slot can exist
// long before anyone is named on it, and the name can change afterwards.
//
// Deliberately a profile rather than a bare name: presentation will grow (avatars, account
// identity, connected-or-not), and each of those becomes a field here rather than a new channel.
export type Profile = {
  // null = the slot exists but nobody has named it. Render the actor key.
  display_name: string | null;
};

// Profiles REPLACE, per actor. Actors not mentioned are untouched.
//
// A profile only ever arrives for an actor whose MapPlayer this connection has ALREADY been sent —
// the server gates it on that, so this channel can never be how you learn someone exists.
export type ProfileUpdate = {
  profiles: [ActorKey, Profile][];
};

export type OutputData =
  | { Batch: Batch }
  | { Profiles: ProfileUpdate };

// Per-connection, strictly increasing by 1 from 1. A gap means desync.
export type ServerOutput = {
  seq_num: number;
  data: OutputData;
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

