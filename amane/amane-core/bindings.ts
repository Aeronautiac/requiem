
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

export type State =
  | "Dead"
  | "Incarcerated"
  | "Ipp"
  | "Kidnapped"
  | "Custody"
  | "UnderTheRadar";
export type States = number;
export const StateFlag = {
  Dead: 1 << 0,
  Incarcerated: 1 << 1,
  Ipp: 1 << 2,
  Kidnapped: 1 << 3,
  Custody: 1 << 4,
  UnderTheRadar: 1 << 5,
} as const;

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

export type LeadershipTransferPolicies = number;
export const LeadershipTransferPolicyFlag = {
  Choose: 1 << 0,
  Random: 1 << 1,
} as const;

export type PoolLinkType = "Restrictive" | "Permissive";

export type ContactLogType = "Full" | "Even" | "Odd";

export type ContactEvent = "LoungeOpened" | "GroupchatAdded" | "GroupchatRemoved";

export type ContactLog = {
  contact_id: number;
  contactor: ActorDisplay;
  contacted: ActorDisplay;
  event: ContactEvent;
};

export type TapInOutcome =
  | { Found: { channel_id: ChannelKey; range: number | null } }
  | "NoSuchContact"
  | "NotLoggable";

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

export type BugContext = "Explicit" | "Custody";

export type ProsecutionSource =
  | "None"
  | { Ability: AbilityKey };

export type TrialSubphaseView = "Grace" | "Presentation";

export type TrialPhaseView =
  | { Prosecutor: TrialSubphaseView }
  | { Defense: TrialSubphaseView }
  | {
      Debate: {
        prosecutor_done: boolean;
        defense_done: boolean;
        awaiting_host: boolean;
      };
    };

export type ProsecutionPhaseView =
  | {
      Custody: {
        prosecutor_ready: boolean;
        defense_ready: boolean;
        awaiting_host: boolean;
      };
    }
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
  | { UnlawfulArrest: { target: ActorKey } }
  | { UnderTheRadar: Record<string, never> }
  | { ShinigamiSacrifice: { sacrifice: ActorKey; name_target: ActorKey } }
  | { KiraConnection: { lounge: LoungeKey } }
  | { TrueNameReroll: { target: ActorKey; true_name: string } }
  | { TapIn: { contact_id: number } }
  | { SilentProsecute: { target: ActorKey } }
  | { PublicKidnap: { target: ActorKey; performer: ActorKey | null } }
  | { AnonymousKidnap: { target: ActorKey } }
  | { Bug: { target: ActorKey } };

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

export type AddToOrg = {
  leader: boolean;
  og: boolean;
  actor_id: ActorKey;
  org_id: ActorKey;
};

export type SetOgStatus = {
  actor_id: ActorKey;
  org_id: ActorKey;
  og: boolean;
};

export type SetBlacklistStatus = {
  actor_id: ActorKey;
  org_id: ActorKey;
  blacklisted: boolean;
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

export type UpdatePassiveVisibilities = Record<string, never>;

export type CreateChannel = {
  loggable: boolean;
};

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

export type AddToGroupchat = {
  groupchat_id: GroupchatKey;
  player_id: ActorKey;
  owner: boolean;
};

export type CreateGroupchat = Record<string, never>;

export type CreatePersonalChannel = Record<string, never>;

export type RemoveFromGroupchat = {
  groupchat_id: GroupchatKey;
  player_id: ActorKey;
};

export type SetGroupchatOwner = {
  groupchat_id: GroupchatKey;
  owner: ActorKey | null;
};

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

export type Null = Record<string, never>;
export type Crash = Record<string, never>;

export type ScheduleJob = {
  timestamp: number;
  payload: Action;
};

export type CreateIncarceration = {
  victim_id: ActorKey;
  source: IncarcerationSource;
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
  verdict: boolean | null;
};

export type Update = Record<string, never>;

export type AddToWorldChannels = {
  player_id: ActorKey;
};

export type CreateOrgs = Record<string, never>;

export type InitializeEngine = {
  seed: number;
};

export type InitializeWorld = Record<string, never>;

export type StartGame = Record<string, never>;

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
  | { StartGame: StartGame }
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
  | { SetOgStatus: SetOgStatus }
  | { SetBlacklistStatus: SetBlacklistStatus }
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
  | { UpdatePassiveVisibilities: UpdatePassiveVisibilities }
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
  | "PerformerRequiresOrg"
  | "NotAnOgMember"
  | "CannotSacrificeForOwnName";

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

export type Command =
  | { EnterViewport: { viewport: ViewportKey; actor: ActorKey } }
  | { ExitViewport: { viewport: ViewportKey; actor: ActorKey } }
  | { MapViewport: { viewport: ViewportKey; kind: ViewportKind } }
  | { Death: { target_id: ActorKey, true_name: string; death_message: string; role: Role; notebook_transferred: boolean; ability_transferred: boolean } }
  | { Kidnapping: { kidnapping_id: KidnappingKey; target_id: ActorKey; duration: number | null } }
  | { KidnapReveal: { kidnapping_id: KidnappingKey; kidnapper: ActorKey | null } }
  | { Incarceration: { incarceration_id: IncarcerationKey; victim_id: ActorKey; duration: number | null } }
  | { IncarcerationReleased: { incarceration_id: IncarcerationKey } }
  | { PseudocideRevival: { target_id: ActorKey } }
  | { AnonymousAnnouncement: { content: string } }
  // The org is a NAME, not a key: this reaches everyone present, and an org's key only resolves for
  // a client that can see that org's channel. Who was accused is deliberately not carried.
  | { FailedSilentProsecution: { accuser_id: ActorKey; true_name: string; org: OrganizationName } }
  | { ActorState: { state: States; actor_id: ActorKey } }
  | { AddOrgMember: { player_id: ActorKey; org_id: ActorKey } }
  | { RemoveOrgMember: { player_id: ActorKey; org_id: ActorKey } }
  | { AddMessage: { content: string; channel_id: ChannelKey; sender_display: ActorDisplay } }
  | { MapActor: { actor_id: ActorKey; kind: ActorKind } }
  | { MapChannel: { channel_id: ChannelKey; kind: ChannelKind } }
  | { ArchiveChannel: { channel_id: ChannelKey } }
  | { SetChannelLoggable: { channel_id: ChannelKey; loggable: boolean } }
  | { NewBug: { bug_key: BugKey } }
  | { AddBugMessage: { bug_key: BugKey; display: ActorDisplay; content: string } }
  | { ArchiveBug: { bug_key: BugKey } }
  | { Bugged: { context: BugContext } }
  | { NewIteration: { iteration: number } }
  | { GcOwnerStatus: { owner: boolean; gc_id: GroupchatKey } }
  | { OgStatus: { target_id: ActorKey; org_id: ActorKey; og: boolean } }
  | { ShowChannelMember: { channel_id: ChannelKey; display: ActorDisplay; channel_perms: ChannelPermissions } }
  | { RemoveChannelMember: { channel_id: ChannelKey; display: ActorDisplay } }
  | { UpdateChannelView: { channel_id: ChannelKey; perms: ChannelPermissions; displays: ActorDisplay[] } }
  | { KiraConnectionAttempt: { channel_id: ChannelKey; user: ActorKey; success: boolean } }
  | { NotebookWrite: { notebook_id: NotebookKey; user_id: ActorKey; message: string | null; true_name: string; delay: number; successes_remaining: number; attempts_remaining: number; success: boolean; target_saved: boolean } }
  | { NotebookBorrowingStatus: { notebook_id: NotebookKey; borrowed: boolean } }
  | { AddContactLog: { passive_id: PassiveKey; log: ContactLog } }
  | { UpdateAbilityView: { ability_name: AbilityName; success_usages_remaining: number; failure_usages_remaining: number; iterations_to_reset: number; ability_id: AbilityKey; owner_id: ActorKey } }
  | { RemoveAbility: { ability_id: AbilityKey } }
  | { UpdatePassiveView: { passive_type: PassiveType; passive_id: PassiveKey; owner_id: ActorKey } }
  | { RemovePassive: { passive_id: PassiveKey } }
  | { RevealAutopsyMessages: { target_id: ActorKey; range: number; redact_names: boolean } }
  | { TapInResult: { contact_id: number; outcome: TapInOutcome } }
  | { ChannelTapped: { channel_id: ChannelKey } }
  | { RevealTrueName: { target_id: ActorKey; true_name: string } }
  | { RevealNotebookHolding: { target_id: ActorKey; holding: boolean } }
  | { RoleUpdate: { target_id: ActorKey; role: Role } }
  | { TrueNameUpdate: { target_id: ActorKey; true_name: string } }
  | { UpdatePoll: { poll_id: PollKey; subject: PollSubject; scope: PollVisibility; accept: number; reject: number; potential: number; opener: ActorKey | null } }
  | { ClosePoll: { poll_id: PollKey; outcome: PollOutcome } }
  | { UpdatePollView: { poll_id: PollKey; eligible: boolean; own_vote: boolean | null } }
  | { UpdateProsecution: { prosecution_id: ProsecutionKey; prosecutor_display: ActorDisplay; defendant_display: ActorDisplay; phase: ProsecutionPhaseView; trial_channel: ChannelKey | null; lawyer_display: ActorDisplay | null } }
  | { CloseProsecution: { prosecution_id: ProsecutionKey; verdict: boolean | null } };

// What a channel belongs to. Every channel in the game is an ordinary engine channel; this is the
// thing that owns it, and it carries whatever ties the two together.
export type ChannelKind =
  | { World: WorldChannelName }
  | { Lounge: { lounge_id: LoungeKey; contact_id: number } }
  | { Groupchat: { gc_id: GroupchatKey; contact_id: number } }
  | { Notebook: NotebookKey }
  // The org's actor id. This is the only statement of which channel backs which org.
  | { Org: ActorKey }
  | "Personal"
  | { Lawyer: ProsecutionKey }
  | { Trial: ProsecutionKey };

// What kind of actor a slot holds. A player carries nothing — the slot existing is all the engine
// has to say, and the display name arrives on the profile channel.
export type ActorKind = "Player" | { Org: OrganizationName };

export type ViewportKind =
  | "Channel"
  | "Bug"
  | "Poll"
  | "Passive"
  | "Presence"
  | "Log";

export type CommandRecipient = "System" | { Actor: ActorKey } | { Viewport: ViewportKey };

export type CommandPayload = {
  timestamp: number;
  recipient: CommandRecipient;
  cmd: Command;
};

export type ActionContext = {
  commands: CommandPayload[];
};

export type IpcExecutionResult =
  | { Ok: [ActionResponse, ActionContext] }
  | { Err: [ActionError, ActionContext] };

export type AppExecResult =
  | { Standard: IpcExecutionResult }
  | "Crashed";

export type AppExecution = {
  exec_result: AppExecResult;
};

export type ServerInput =
  | { Action: ActionRequest }
  | { Control: GameControl };

export type Capability = "Administer" | "Supervise";

export type ActorScope = "All" | { Only: ActorKey[] };

export type GameControl =
  | "EndGame"
  | { CreateKey: { actors: ActorScope; capabilities: Capability[] } }
  | { RevokeKey: { key: string } }
  | { SetCapabilities: { key: string; capabilities: Capability[] } }
  | { SetActorScope: { key: string; actors: ActorScope } }
  | { SetProfile: { actor: ActorKey; profile: Profile } };

export type ControlResponse =
  | "Ended"
  | "KeyRevoked"
  | "CapabilitiesSet"
  | "ActorScopeSet"
  | "ProfileSet"
  | { KeyCreated: { key: string } };

export type ControlError =
  | "KeyNotFound"
  | "CannotActOnSelf"
  | "RequiresSupervise"
  | "CannotGrantSupervise";

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

export type ResponsePair = {
  input: ServerInput;
  output: ExecOutcome;
};

export type Batch = {
  commands: CommandPayload[];
  response: ResponsePair | null;
};

export type Profile = {
  display_name: string | null;
};

export type ProfileUpdate = {
  profiles: [ActorKey, Profile][];
};

// What this connection's own key permits. Arrives before anything else and again whenever it is
// rewritten, so the UI never has to infer its own standing from which commands happened to show up.
// UX only — the server checks every action and control against the ledger regardless.
export type PrivilegeSet = {
  actors: ActorScope;
  capabilities: Capability[];
};

export type OutputData =
  | { Batch: Batch }
  | { Profiles: ProfileUpdate }
  | { Privileges: PrivilegeSet };

export type ServerOutput = {
  seq_num: number;
  data: OutputData;
};

export function slotKeyToString(key: SlotKey): string {
  return `${key.idx}:${key.version}`;
}

export function slotKeyFromString(s: string): SlotKey {
  const [idx, version] = s.split(":").map(Number);
  return { idx, version };
}

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

