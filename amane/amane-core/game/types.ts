// The shapes a view holds. No behaviour and no state — just what the command handlers build and
// the components read.
import type {
  AbilityName,
  ActorDisplay,
  BugContext,
  ChannelProfileView,
  ContactLog,
  LogCommand,
  LogType,
  OrgAbility,
  ProfileOwners,
  OrganizationName,
  PassiveType,
  PollOptionIndex,
  PollOptionTally,
  PollOutcome,
  PollParent,
  PollSubject,
  ProsecutionPhaseView,
  Role,
  TapInOutcome,
} from "../bindings";

export type WorldEvent = {
  // Iteration 1 is the host starting the game, so this is also how a viewer learns play has begun.
  NewIteration: {
    iteration: number,
  }
} | {
  // The world went dark, or came back. Rides world data rather than the events viewport it
  // silences, so it is the one thing a viewer still hears during a blackout — without it, silence
  // would be indistinguishable from nothing happening.
  Blackout: {
    active: boolean,
  }
} | {
  Death: {
    target_id: string,
    true_name: string,
    death_message: string,
    role: Role,
    notebook_transferred: boolean,
    ability_transferred: boolean,
  }
} | {
  PseudocideRevival: {
    target_id: string,
  }
} | {
  Kidnapping: {
    kidnapping_id: string,
    target_id: string,
    duration: number | null, // null = indefinite
  }
} | {
  KidnapReveal: {
    kidnapping_id: string,
    victim: string | null, // resolved from the tracked kidnapping (null = unknown)
    kidnapper: string | null, // null = stayed anonymous
  }
} | {
  Incarceration: {
    incarceration_id: string,
    victim_id: string,
    duration: number | null, // null = held until released
  }
} | {
  // Who ordered the incarceration is never disclosed, so unlike KidnapReveal there is nothing
  // here to leak.
  IncarcerationReleased: {
    incarceration_id: string,
    victim: string | null,
  }
} | {
  AnonymousAnnouncement: {
    content: string,
  }
} | {
  // Someone made the shinigami eye deal, revealed only as a display (currently their role).
  EyeDealTaken: {
    user: ActorDisplay,
  }
} | {
  // A press-conference roster change: someone gained or lost the right to speak on the news.
  PressConfStatus: {
    target_id: string,
    has_access: boolean,
  }
} | {
  // Who now holds the news anchor post, or null when it was vacated.
  NewsAnchor: {
    target_id: string | null,
  }
} | {
  // A silent prosecution named somebody who was not wanted. The accuser is the only person in it —
  // who they accused is never carried, so there is nothing here to resolve against `players`.
  FailedSilentProsecution: {
    accuser_id: string,
    true_name: string,
    org: OrganizationName,
  }
} | {
  // Derived by diffing the view's own prosecution snapshot, or from CloseProsecution. `phase` is
  // the one being entered; on `ended` it's the last phase seen. `verdict` is only set on `ended`:
  // true guilty, false acquitted, null for a prosecution that ended some other way.
  ProsecutionEvent: {
    prosecution_id: string,
    prosecutor_display: ActorDisplay,
    defendant_display: ActorDisplay,
    phase: ProsecutionPhaseView,
    ended: boolean,
    verdict: boolean | null,
  }
}

export type WriteEvent = {
  user_id: string,
  notebook_id: string,
  message: string,
  true_name: string,
  delay: number,
  successes_remaining: number,
  attempts_remaining: number,
  success: boolean,
  target_saved: boolean,
}

// The three bits the engine sends, and nothing derived.
//
// No history cutoff belongs here. The engine's `Channel::viewers()` is exactly "everyone with
// View", so losing read exits the membership viewport and this view stops being delivered the
// channel's content. Delivery is the window.
export type ChannelPerms = {
  read: boolean;
  send: boolean;
  loggability_control: boolean;
}

// A channel carries two orthogonal axes: `category` is WHERE it renders in the sidebar (grouping
// only, no behaviour), `kind` is HOW it behaves. The split lets the read-only Notifications feed
// (kind "Info") sit in the same "Personal" category as sendable personal channels (kind "Standard").
export type ChannelCategory =
  | "Raw" | "Lounge" | "Groupchat" | "Notebook" | "Role" | "World" | "Org" | "Prosecution" | "Logs" | "Personal" | "Kidnapping";
export const CHANNEL_CATEGORIES: ChannelCategory[] = [
  "Raw", "Lounge", "Groupchat", "Notebook", "Role",
  "World", "Org", "Prosecution", "Logs", "Personal",
];

// Only properties that can't be derived elsewhere live here — a channel that merely has an
// associated object (a notebook, a group chat) stays "Standard" and is recognised via its mapping,
// not a dedicated kind.
//   - "Info":       frontend-only feed, built by the client rather than delivered.
//   - "Bug":        a bug's relayed messages.
//   - "ContactLog": a contact-log passive's record.
export type ChannelKind = "Standard" | "Info" | "Bug" | "ContactLog" | "Log";

export type Channel = {
  kind: ChannelKind;
  category: ChannelCategory;
  name: string;
  archived: boolean;
  events: GameEvent[];
}

export type Message = {
  sender_display: ActorDisplay,
  content: string,
}

// Entries that render inside a read-only Info channel. Kept separate from WorldEvent so a directed
// personal event can never leak into the news stream.
export type InfoEvent = {
  RevealTrueName: {
    target_id: string,
    true_name: string,
  }
} | {
  RevealNotebookHolding: {
    target_id: string,
    holding: boolean,
  }
} | {
  // `context` says why (explicit ability vs custody); who planted it is intentionally unknown.
  Bugged: {
    context: BugContext,
  }
} | {
  RoleUpdate: {
    role: Role,
  }
} | {
  // Derived, not delivered: the public NewsAnchor names the anchor to everyone, and the client
  // raises this for itself when that name is (or stops being) its own key. `holding` is which way.
  NewsAnchorStatus: {
    holding: boolean,
  }
} | {
  // Derived like NewsAnchorStatus: the public PressConfStatus names the actor to everyone, and the
  // client raises this for itself when that actor is its own key. `in_conf` is which way.
  PressConfMembership: {
    in_conf: boolean,
  }
} | {
  TrueNameUpdate: {
    true_name: string,
  }
} | {
  // Derived from gaining read access to a notebook channel; no engine command backs it.
  NotebookReceived: Record<string, never>,
} | {
  // A private answer to a private question: the tapped channel is told separately, and is never
  // told who tapped it.
  TapInResult: {
    contact_id: number,
    outcome: TapInOutcome,
  }
} | {
  // Directed: you gained or lost leadership of one org. `org_id` names which, resolved at render
  // time — a member can lead more than one org, so the bare flag would be ambiguous.
  LeaderStatus: {
    org_id: string,
    leader: boolean,
  }
} | {
  // Directed: your own eye count after an ability changed it (e.g. a failed notebook reveal spending
  // a volatile eye). Only sent on a change.
  EyeCount: {
    count: number,
  }
}

// A poll started (outcome null) or ended, rendered inline in the poll's scoped channel. Distinct
// from the Polls panel, which is where you actually vote.
export type PollNoticeEvent = {
  PollNotice: {
    poll_id: string,
    subject: PollSubject,
    outcome: PollOutcome | null,
    // Actor KEY, always null on a close notice. Resolved at render time — see PollData.opener.
    opener: string | null,
  }
}

// The engine's ContactLog verbatim — both ends are displays, so nothing here can be turned back
// into a player.
export type ContactLogEvent = {
  ContactLogEntry: ContactLog
}

// Someone reached for Kira through a lounge. Emitted whether or not they found him, and the user
// is named raw — you cannot feel for Kira quietly.
export type KiraConnectionEvent = {
  KiraConnectionAttempt: {
    // Actor KEY, resolved at render time (see PollData.opener for why).
    user: string,
    success: boolean,
  }
}

// This channel's record was read. Carries nothing: who tapped is exactly what is withheld.
export type ChannelTappedEvent = {
  ChannelTapped: Record<string, never>
}

export type GameEvent = {
  timestamp: number,
  data:
  | { Message: Message }
  | { Write: WriteEvent }
  | WorldEvent
  | InfoEvent
  | PollNoticeEvent
  | ContactLogEvent
  | KiraConnectionEvent
  | ChannelTappedEvent
  // Client-only. A Death command is staged into a run of timed reveals (see stage_world_events);
  // these are the later beats. The engine never sends them — they are derived from the one Death.
  | { DeathRole: { target_id: string, role: Role } }
  | { DeathOrgs: { target_id: string, orgs: { id: string, leader: boolean, og: boolean }[] } }
  | { DeathTransfer: { target_id: string, notebook_transferred: boolean, ability_transferred: boolean } },
}

export type PollData = {
  subject: PollSubject,
  parent: PollParent,
  // The choices and the weight behind each, in the order they are offered. A vote names one by
  // its position here.
  options: PollOptionTally[],
  potential: number,
  // Actor KEY, not a name. Resolved at render time: a name resolved at apply time would be
  // whatever `players` held then, and a replay later would resolve it differently.
  opener: string | null,
  // Set once resolved. The entry is KEPT rather than deleted: a poll rides its parent's viewport,
  // so a view gaining that viewport replays every poll the parent ever held and has to reach the
  // same place. Live polls filter on null.
  outcome: PollOutcome | null,
}

// Having an entry at all means the viewer can see the poll; `eligible` is whether they may vote.
export type PollView = {
  eligible: boolean,
  own_vote: PollOptionIndex | null,
}

// The trial channel and verdict poll ride their own command streams; trial_channel is just the id
// so the UI can tag that channel as a prosecution channel.
export type ProsecutionData = {
  prosecutor_display: ActorDisplay,
  defendant_display: ActorDisplay,
  // Public — a trial's defence counsel is not a hidden fact.
  lawyer_display: ActorDisplay | null,
  phase: ProsecutionPhaseView,
  trial_channel: string | null,
}

export interface AbilityView {
  name: AbilityName;
  // Split by outcome: conditional charge subtraction means successful and failed uses can have
  // different remaining counts (a true-name guess bounded by an attempts pool on failure but an
  // invite pool on success).
  success_usages_remaining: number;
  failure_usages_remaining: number;
  iterations_to_reset: number;
  // The recharge period (config), so the cadence shows before any use; iterations_to_reset is the
  // live countdown, 0 until a use arms it.
  base_reset: number;
  // No charge pools at all — a pool IS the restriction, so its absence means unlimited use. When
  // set, the counts and reset fields above are meaningless and the UI ignores them.
  unlimited: boolean;
  // The static gates on an ORG ability — arrives on a separate OrgAbilityRequirements command after
  // the view itself. Undefined for a personal ability, which has no such gates.
  requirements?: OrgAbility;
}

// The type itself may carry data (VoteAmplification's multiplier), so this is the full PassiveType.
export interface PassiveView {
  type: PassiveType;
}

// A channel as this view stands in it. Both halves are whole sets, replaced rather than patched:
// they are current state, not a sequence of events, which is why the engine directs them here
// instead of addressing them to the channel's viewport.
export type ChannelView = {
  // Every name the room can see, and what each may do. Per-view because the same actor can be
  // shown under different names to different viewers (deception), and because a name the room has
  // not been told about is simply absent — its existence is the thing being kept.
  roster: ChannelProfileView[];
  // Every name here that is THIS view's to speak as, whether or not the room can see it. Almost
  // always exactly one. Empty is a member who holds nothing, which is how being removed is stated
  // rather than left to be noticed.
  own: ChannelProfileView[];
  // Who is behind each name in the roster, keyed by profile_id. Only ever populated for the admin
  // (System) view — the engine sends ownership to nobody else, so for a player this stays empty.
  owners: ProfileOwners[];
};

// A player slot this view has been told about (from MapActor).
//
// The slot comes first and always; the NAME is a separate server-level fact that may never arrive.
// So `display_name` is nullable, and every render site goes through `playerLabel` rather than
// reaching for the field — an unnamed slot is a normal, permanent state, not a missing value.
export interface Player {
  display_name: string | null;
}

// Populated from the System copy of the personal-info commands, so this stays empty on every view
// but System's.
export interface PlayerInfo {
  role?: Role;
  true_name?: string;
  // OG standing goes to the member and to System, and to nobody else in the org.
  og_orgs?: Set<string>;
}

// Members and abilities arrive on the org channel's viewport: a viewer is shown the org iff they
// can see that channel, and all members see the same list.
export interface Org {
  name: OrganizationName;
  leader: string | null; // actor key or nothing
  members: Set<string>; // dead members included
  // The present subset of `members` — those who count toward the org's ability member
  // requirements. A member who has lost presence (kidnapped, jailed, dead) stays in `members` but
  // drops out of here. Whole set, replaced by OrgEffectiveMembers.
  effective: Set<string>;
  abilities: Map<string, AbilityView>;
}

// Kept after its reveal rather than deleted: the reveal command carries only the kidnapping's id,
// so resolving the victim is a lookup here, and a replay later must resolve the same one.
export type TrackedKidnapping = {
  victim: string;
  duration: number | null; // null = indefinite
  revealed: boolean;
};

// Kept for the same reason: the release command carries only the id.
export type TrackedIncarceration = {
  victim: string;
  duration: number | null; // null = held until released
  released: boolean;
};

