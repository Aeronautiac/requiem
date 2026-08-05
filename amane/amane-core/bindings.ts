
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
export type ProfileKey = SlotKey;
// A record's identity, and a plain counter rather than a slot: a record is never freed, so there
// is no generation to guard against reuse.
export type LogID = number;

export type Role =
  | "Kira"
  | "SecondKira"
  | "L"
  | "Watari"
  | "BeyondBirthday"
  | "PrivateInvestigator"
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
  | "ShinigamiEyeDeal"
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

// The public projection of an actor's condition, as opposed to States (the raw set only the actor
// itself is told). Not a subset of States: `Bugged` is a bug object, not a state, and `Missing` is
// the blackout blur — a NEW presence loss shows only as Missing, so display it as a vague "gone"
// and never infer the specific reason. UnderTheRadar is deliberately never in here.
export type Status =
  | "Bugged"
  | "Dead"
  | "Incarcerated"
  | "Kidnapped"
  | "Custody"
  | "Ipp"
  | "Missing";
export type Statuses = number;
export const StatusFlag = {
  Bugged: 1 << 0,
  Dead: 1 << 1,
  Incarcerated: 1 << 2,
  Kidnapped: 1 << 3,
  Custody: 1 << 4,
  Ipp: 1 << 5,
  Missing: 1 << 6,
} as const;

export type ChannelPermissions = number;
export const ChannelPermissionFlag = {
  Send: 1 << 0,
  View: 1 << 1,
  LoggabilityControl: 1 << 2,
} as const;

// One profile: who someone may appear as in a channel, and what appearing as them may do.
//
// A profile is the unit of participation, so this is what both halves of the channel protocol
// carry — the visible ones go to the room on ChannelRoster, and your own come to you on
// ProfileAccess. The two are separate because you may hold a name before the room is told it
// exists, and the display of an unrevealed name is exactly what must not go to the room.
export type ChannelProfileView = {
  profile_id: ProfileKey;
  display: ActorDisplay;
  perms: ChannelPermissions;
};

// SYSTEM-only. Who wears one name in a channel — the admin's view behind the roster. Pairs with a
// ChannelProfileView by profile_id; an empty owners list is a name currently worn by nobody.
export type ProfileOwners = {
  profile_id: ProfileKey;
  owners: ActorKey[];
};

// A profile's permission rule, evaluated by the engine after every action. The client never
// applies one — it carries them because the admin setup actions name them.
export type PermUpdatePolicy =
  | { Fixed: { perms: ChannelPermissions } }
  | { Contact: Record<string, never> }
  | { News: Record<string, never> }
  | { Presence: { perms: ChannelPermissions } }
  | { Alive: Record<string, never> }
  | { Trial: { prosecution_id: ProsecutionKey } };

export type BlueprintDisplayKind = "OwnerRaw";

// The profile every player is handed a copy of, and what makes them a member at all. A channel
// without one has its membership decided by whatever action owns it.
export type ProfileBlueprint = {
  start_visible: boolean;
  perm_policy: PermUpdatePolicy;
  display_kind: BlueprintDisplayKind;
};

export type OrganizationName = "NULL" | "KK" | "TF" | "SPK";

// One member's standing in an org, as revealed on death and as fabricated by a pseudocide.
export type OrgMemberView = { leader: boolean; og: boolean };

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
  // There is a record to read. `range` is how far back from now, or null for everything it ever
  // held. The log rather than the channel, so answering a tap-in needs no model of which channel
  // owns what.
  | { Found: { log: LogID; range: number | null } }
  // Nothing has ever been registered under that id.
  | "NoSuchContact"
  // The channel is real, but logging is off there, so nothing was ever written down.
  | "NotLoggable";

export type PassiveType =
  | "Wanted"
  | { VoteAmplification: { multiplier: number } }
  | "VolatileEyes"
  | { ContactLogs: ContactLogType }
  | "OwnedNotebookBlock"
  | "CustodyBugReceiver"
  | "NewsControl"
  | "NewsAccess";

export type VoterPolicy = "Present";

export type PollPolicy = "AlwaysInconclusive" | "Majority" | "MostVoted";

// What a poll hangs off. Its membership is who the poll is put to; its viewport is who can reach
// the ballot right now, and a poll is addressed to that viewport rather than owning one.
export type PollParent =
  | { Org: ActorKey }
  | { Channel: ChannelKey }
  | "World";

export type PollSubject =
  | { OrgAbility: AbilityBehaviour }
  | { CivilianArrest: ActorKey }
  | { Generic: string };

// Which option a vote is for: an index into the poll's options, fixed for the poll's life.
export type PollOptionIndex = number;

export type PollOptionLabel = "Accept" | "Reject" | { Generic: string };

export type PollOptionTally = {
  label: PollOptionLabel;
  weight: number;
};

export type PollOutcome =
  | { Resolved: PollOptionIndex }
  | "Inconclusive"
  | "Cancelled";

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

// Which side of a prosecution you are on, told to you privately. Counsel is absent because a
// lawyer is named publicly on the snapshot and always shown raw — there was never anything to tell.
export type ProsecutionSide = "Prosecutor" | "Defendant";

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

export type AbilityBehaviour =
  | { Contact: { target_id: ActorKey } }
  | { Pseudocide: { target_id: ActorKey; true_name: string; death_message: string | null; role: Role; orgs: [ActorKey, OrgMemberView][]; notebook_transferred: boolean; ability_transferred: boolean } }
  | { Gun: { target_id: ActorKey } }
  | { AnonymousAnnouncement: { content: string } }
  | { AnonymousContact: { target: ActorKey } }
  | { CreateGroupchat: Record<string, never> }
  | { FabricateLounge: { contacted_id: ActorKey; contactor_id: ActorKey } }
  | { FalseAnonymousContact: { target: ActorKey; role: Role } }
  | { Ipp: { target: ActorKey } }
  | { Prosecute: { target: ActorKey } }
  | { AnonymousProsecute: { target: ActorKey } }
  | { Autopsy: { target: ActorKey } }
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
  | { ShinigamiEyeDeal: Record<string, never> }
  | { KiraConnection: { lounge: LoungeKey } }
  | { TrueNameReroll: { target: ActorKey; true_name: string } }
  | { TapIn: { contact_id: number } }
  | { SilentProsecute: { target: ActorKey } }
  | { Blackout: Record<string, never> }
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

export type UpdateContactLogViewports = Record<string, never>;

export type CreateChannel = {
  loggable: boolean;
  // Set for a channel everyone belongs to; null for one whose membership some action owns.
  base_profile: ProfileBlueprint | null;
};

export type DestroyChannel = {
  channel_id: ChannelKey;
};

export type SendMessage = {
  channel_id: ChannelKey;
  // Who to say it as. A player must name a profile they own, and Send belongs to the profile
  // rather than to the sender: the same person may be able to speak here as one of their names and
  // not as another.
  //
  // null is an admin speaking as nobody, which shows as ActorDisplay::System.
  profile_id: ProfileKey | null;
  content: string;
};

export type SetLoggable = {
  channel_id: ChannelKey;
  loggable: boolean;
};

export type SetNotebookFake = {
  notebook_id: NotebookKey;
  fake: boolean;
};

export type SetTrueName = {
  target_id: ActorKey;
  true_name: string;
};

// Membership is not set, it follows. You are a member of a channel exactly while you own a profile
// in it, so these are the whole of joining and leaving as well as of who may appear as what.

// Put a name into a channel. It starts owned by nobody; handing it out is SetProfileAccess.
export type AddProfile = {
  channel_id: ChannelKey;
  display: ActorDisplay;
  // False for a name the room does not know yet. It is absent from the roster until its first
  // message reveals it.
  visible: boolean;
  // Whether more than one actor may hold it at once.
  shared: boolean;
  // Whether it could ever belong to somebody other than whoever holds it now. A name that could
  // not is destroyed along with its holder's membership.
  transferrable: boolean;
  perm_policy: PermUpdatePolicy;
};

// Put a name into a channel and hand it straight to somebody, which is also what makes them a
// member. By far the common shape.
export type CreateAndGiveProfile = {
  channel_id: ChannelKey;
  player_id: ActorKey;
  display: ActorDisplay;
  visible: boolean;
  shared: boolean;
  transferrable: boolean;
  perm_policy: PermUpdatePolicy;
};

export type SetProfilePolicy = {
  channel_id: ChannelKey;
  profile_id: ProfileKey;
  perm_policy: PermUpdatePolicy;
};

export type DeleteProfile = {
  channel_id: ChannelKey;
  profile_id: ProfileKey;
};

// Take a player out of a channel entirely, disposing of each of their names by whether it could
// ever have belonged to anybody else.
export type RemoveFromChannel = {
  channel_id: ChannelKey;
  player_id: ActorKey;
};

// Give a player a profile, or take it away. A first profile makes them a member; losing their last
// ends the membership.
export type SetProfileAccess = {
  channel_id: ChannelKey;
  profile_id: ProfileKey;
  player_id: ActorKey;
  granted: boolean;
};

export type UpdateChannels = Record<string, never>;

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

export type Null = Record<string, never>;
export type Crash = Record<string, never>;
export type NextIteration = Record<string, never>;

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

// Reconcile who is in the prison channel against who is actually locked up. The prison hands out
// no seats of its own, so its membership is managed and the incarceration state is the authority.
export type UpdatePrisonChannel = Record<string, never>;

export type CreateKidnapping = {
  victim_id: ActorKey;
  kidnapping_type: KidnappingType;
  source: KidnappingSource;
  duration: number | null;
};

export type CullKidnappings = {
  ability_id: AbilityKey;
};

export type ReleaseKidnapping = {
  kidnapping_id: KidnappingKey;
  forced: boolean;
};

// Re-derive who is on the captors' side of every active kidnapping, and seat them. A sweep over
// KIDNAPPINGS rather than channels: who the captors are is a property of what started the
// kidnapping, and an org's roster moves while somebody is still being held.
export type UpdateKidnappings = Record<string, never>;

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
  option: PollOptionIndex;
};

export type PollOption = {
  label: PollOptionLabel;
  payload: Action | null;
};

export type CreatePoll = {
  voter_policy: VoterPolicy;
  parent: PollParent;
  subject: PollSubject;
  update_policy: PollPolicy;
  timeout_policy: PollPolicy;
  options: PollOption[];
  ignore_amplification: boolean;
  duration: number | null;
  opener: ActorKey | null;
};

export type PollCleanup = {
  poll_id: PollKey;
  outcome: PollOutcome;
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

export type CreateOrgs = Record<string, never>;

export type InitializeEngine = {
  seed: number;
};

export type InitializeWorld = Record<string, never>;

export type StartGame = Record<string, never>;

export type SetRandomSeed = {
  seed: number;
};

export type UpdateWorldViewports = Record<string, never>;

export type SetBlackout = {
  active: boolean;
};

export type PressConfAccess = {
  target_id: ActorKey;
  has_access: boolean;
};

// null vacates the post.
export type SetNewsAnchor = {
  target_id: ActorKey | null;
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
  | { AddProfile: AddProfile }
  | { CreateAndGiveProfile: CreateAndGiveProfile }
  | { DeleteProfile: DeleteProfile }
  | { RemoveFromChannel: RemoveFromChannel }
  | { SetProfileAccess: SetProfileAccess }
  | { SetProfilePolicy: SetProfilePolicy }
  | { UpdateChannels: UpdateChannels }
  | { SetLoggable: SetLoggable }
  | { SetNotebookFake: SetNotebookFake }
  | { SetTrueName: SetTrueName }
  | { CreateLounge: CreateLounge }
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
  | { UpdateWorldViewports: UpdateWorldViewports }
  | { SetBlackout: SetBlackout }
  | { InitializeEngine: InitializeEngine }
  | { SetRandomSeed: SetRandomSeed }
  | { UpdateBugVisibilities: UpdateBugVisibilities }
  | { UpdateContactLogViewports: UpdateContactLogViewports }
  | { ProsecutionVoteRes: ProsecutionVoteRes }
  | { CreateKidnapping: CreateKidnapping }
  | { ReleaseKidnapping: ReleaseKidnapping }
  | { CullKidnappings: CullKidnappings }
  | { UpdateKidnappings: UpdateKidnappings }
  | { UpdatePrisonChannel: UpdatePrisonChannel }
  | { CreateIncarceration: CreateIncarceration }
  | { ReleaseIncarceration: ReleaseIncarceration }
  | { CullIncarcerations: CullIncarcerations }
  | { NextIteration: NextIteration }
  | { PressConfAccess: PressConfAccess }
  | { SetNewsAnchor: SetNewsAnchor };

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
  | "NoDuplicateOrgs"
  | "ActorAlreadyInOrg"
  | "UserNotPresent"
  | "PlayerNotInOrg"
  | "AlreadyLeader"
  | "ChannelDoesntExist"
  | "NotAChannelMember"
  | "ProfileNotFound"
  | "ProfileNotOwned"
  | "ProfileNotShareable"
  | "ProfileRequired"
  | "IncompatiblePolicy"
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
  | "CannotSacrificeForOwnName"
  | "CannotProsecuteSelf"
  | "GameAlreadyStarted"
  | "GameNotStarted"
  | "MustChooseSuccessor"
  | "NoEyes"
  | "NotAPollOption"
  | "PollHasNoOptions"
  | "ConferenceFull"
  | "AlreadyInConference"
  | "NotInConference"
  | "NoNewsControl"
  | "AlreadyNewsAnchor";

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
  | { AddProfile: { profile_id: ProfileKey } }
  | { CreateAndGiveProfile: { profile_id: ProfileKey } }
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
  | { Death: { target_id: ActorKey, true_name: string; death_message: string; role: Role; orgs: [ActorKey, OrgMemberView][]; notebook_transferred: boolean; ability_transferred: boolean } }
  | { Kidnapping: { kidnapping_id: KidnappingKey; target_id: ActorKey; duration: number | null } }
  | { KidnapReveal: { kidnapping_id: KidnappingKey; kidnapper: ActorKey | null } }
  | { Incarceration: { incarceration_id: IncarcerationKey; victim_id: ActorKey; duration: number | null } }
  | { IncarcerationReleased: { incarceration_id: IncarcerationKey } }
  | { PseudocideRevival: { target_id: ActorKey } }
  | { AnonymousAnnouncement: { content: string } }
  // A press-conference roster change: someone gained or lost the right to speak on the news beyond
  // the anchor. Rides world-events like the rest of the news.
  | { PressConfStatus: { target_id: ActorKey; has_access: boolean } }
  // Who now holds the news anchor post, or null when it is vacant.
  | { NewsAnchor: { target_id: ActorKey | null } }
  // The org is a NAME, not a key: this reaches everyone present, and an org's key only resolves for
  // a client that can see that org's channel. Who was accused is deliberately not carried.
  | { FailedSilentProsecution: { accuser_id: ActorKey; true_name: string; org: OrganizationName } }
  | { ActorState: { state: States; actor_id: ActorKey } }
  | { ActorStatus: { actor_id: ActorKey; status: Statuses } }
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
  | { Blackout: { active: boolean } }
  | { EyeDealTaken: { user: ActorDisplay } }
  | { GcOwnerStatus: { owner: boolean; gc_id: GroupchatKey } }
  | { OgStatus: { target_id: ActorKey; org_id: ActorKey; og: boolean } }
  // DIRECTED: every name the room can currently see, and what each may do. The whole set, every
  // time, sent to each viewer when it changes and to anyone the moment they gain sight of it.
  //
  // Directed and whole rather than addressed to the channel's viewport, and that is the entire
  // point: a viewport replays its history to anyone who enters, so a roster delivered that way
  // would hand every new arrival every name the channel has ever held — the previous holder of a
  // notebook, everyone who was ever in a lounge, every mask worn at a trial.
  //
  // Invisible profiles are absent from it. Their existence is the thing being kept.
  | { ChannelRoster: { channel_id: ChannelKey; profiles: ChannelProfileView[] } }
  // SYSTEM only. The ownership behind a channel's roster: for every visible profile, which actors
  // wear it. Rides System alongside that channel's ChannelRoster and reaches nobody else — seeing
  // through a name to the person is the admin power ordinary viewers lack. Keyed by profile_id.
  | { ProfileOwnership: { channel_id: ChannelKey; owners: ProfileOwners[] } }
  // DIRECTED: which profiles in this channel you may speak as, whether or not the room can see
  // them, and what each permits. Says nothing about whether you can READ the channel — that is the
  // viewport's answer. An empty set is a member who holds nothing here.
  | { ProfileAccess: { channel_id: ChannelKey; profiles: ChannelProfileView[] } }
  | { KiraConnectionAttempt: { channel_id: ChannelKey; user: ActorKey; success: boolean } }
  | { NotebookWrite: { notebook_id: NotebookKey; user_id: ActorKey; message: string | null; true_name: string; delay: number; successes_remaining: number; attempts_remaining: number; success: boolean; target_saved: boolean } }
  | { NotebookBorrowingStatus: { notebook_id: NotebookKey; borrowed: boolean } }
  | { NotebookFakeStatus: { notebook_id: NotebookKey; fake: boolean } }
  | { OrgEffectiveMembers: { org_id: ActorKey; members: ActorKey[] } }
  | { AddContactLog: { kind: ContactLogType; log: ContactLog } }
  | { UpdateAbilityView: { ability_name: AbilityName; success_usages_remaining: number; failure_usages_remaining: number; iterations_to_reset: number; base_reset: number; unlimited: boolean; ability_id: AbilityKey; owner_id: ActorKey } }
  | { RemoveAbility: { ability_id: AbilityKey } }
  | { OrgAbilityRequirements: { ability_id: AbilityKey; requirements: OrgAbility } }
  | { UpdatePassiveView: { passive_type: PassiveType; passive_id: PassiveKey; owner_id: ActorKey } }
  | { RemovePassive: { passive_id: PassiveKey } }
  // The record to read rather than whose it was: the log stores the raw messages, so answering
  // this needs no model of which player or channel owns what.
  | { RevealAutopsyMessages: { log: LogID; range: number; redact_names: boolean } }
  | { TapInResult: { contact_id: number; outcome: TapInOutcome } }
  | { ChannelTapped: { channel_id: ChannelKey } }
  | { RevealTrueName: { target_id: ActorKey; true_name: string } }
  | { RevealNotebookHolding: { target_id: ActorKey; holding: boolean } }
  | { RoleUpdate: { target_id: ActorKey; role: Role } }
  | { TrueNameUpdate: { target_id: ActorKey; true_name: string } }
  | { UpdatePoll: { poll_id: PollKey; subject: PollSubject; parent: PollParent; options: PollOptionTally[]; potential: number; opener: ActorKey | null } }
  | { ClosePoll: { poll_id: PollKey; outcome: PollOutcome } }
  | { UpdatePollView: { poll_id: PollKey; eligible: boolean; own_vote: PollOptionIndex | null } }
  | { UpdateProsecution: { prosecution_id: ProsecutionKey; prosecutor_display: ActorDisplay; defendant_display: ActorDisplay; phase: ProsecutionPhaseView; trial_channel: ChannelKey | null; lawyer_display: ActorDisplay | null } }
  | { InProsecution: { prosecution_id: ProsecutionKey; side: ProsecutionSide } }
  | { CloseProsecution: { prosecution_id: ProsecutionKey; verdict: boolean | null } }
  // admin directed
  | { OrgLeader: { leader: ActorKey | null; org_id: ActorKey } }
  // player directed
  | { LeaderStatus: { org_id: ActorKey; leader: boolean } }
  // player directed: the recipient's own eye count, emitted when an ability changes it
  | { EyeCount: { count: number } };


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
  | { Trial: ProsecutionKey }
  | { Kidnapping: KidnappingKey };

// What kind of actor a slot holds. A player carries nothing — the slot existing is all the engine
// has to say, and the display name arrives on the profile channel.
export type ActorKind = "Player" | { Org: OrganizationName };

// Every variant here is an audience. The record is not one, and is not a viewport at all — see
// LogID.
export type ViewportKind =
  | "Channel"
  | "Bug"
  | "ContactLog"
  | "WorldEvents"
  | "WorldData";

// Log is here for completeness of the protocol only. Nothing addressed to a record is ever
// delivered to any client, admin included, so a client never observes one.
export type CommandRecipient =
  | "System"
  | { Actor: ActorKey }
  | { Viewport: ViewportKey }
  | { Log: LogID };

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

