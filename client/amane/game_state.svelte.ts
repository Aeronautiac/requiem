import { SvelteMap, SvelteSet } from "svelte/reactivity";
import type { AbilityKey, AbilityName, ActorDisplay, BugContext, BugKey, CommandPayload, CommandRecipient, NotebookKey, OrganizationName, PassiveType, PollOutcome, PollSubject, PollVisibility, ProfileUpdate, ProsecutionKey, ProsecutionPhaseView, Role } from "./bindings";
import { slotKeyFromString, slotKeyToString } from "./bindings";

// store all messages and events in the top level, but give every view a copy 

export type WorldEvent = {
  Death: {
    target_id: string,
    true_name: string,
    death_message: string,
    role: Role,
    notebook_transferred: boolean,
    ability_transferred: boolean,
  }
} |
{
  PseudocideRevival: {
    target_id: string,
  }
} | {
  Kidnapping: {
    kidnapping_id: string,
    target_id: string,
    duration: number | null, // null = indefinite (no scheduled auto-release)
  }
} | {
  KidnapReveal: {
    kidnapping_id: string,
    victim: string | null, // resolved from the tracked kidnapping at handle time (null = unknown)
    kidnapper: string | null, // null = the kidnapper stayed anonymous
  }
} | {
  Incarceration: {
    incarceration_id: string,
    victim_id: string,
    duration: number | null, // null = held until released
  }
} | {
  // The prisoner is out. Who ordered the incarceration is never disclosed, so unlike KidnapReveal
  // there is nothing to leak here — only the victim, resolved from the tracked incarceration.
  IncarcerationReleased: {
    incarceration_id: string,
    victim: string | null,
  }
} | {
  AnonymousAnnouncement: {
    content: string,
  }
} | {
  // A prosecution started, advanced a phase, or ended — derived on the frontend by diffing the
  // per-view prosecution snapshot (start/advance) or from CloseProsecution (ended). phase is the
  // phase being entered; on `ended` it's the last phase seen.
  ProsecutionEvent: {
    prosecution_id: string,
    prosecutor_display: ActorDisplay,
    defendant_display: ActorDisplay,
    phase: ProsecutionPhaseView,
    ended: boolean,
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


export type ChannelPerms = {
  read: boolean;
  read_updated: number; // the time that read perms were last updated
  send: boolean;
  loggability_control: boolean;
  had_positive: boolean; // if any perm here has been positive, this is set permanently
}

// A channel carries two orthogonal axes:
//   - `category`: WHERE it renders in the sidebar (grouping/heading only, no behaviour).
//   - `kind`:     HOW it behaves, independent of where it's shown. A kind confers inherent
//                 properties (read-only, non-interactable, …) that hold in any category.
// This split lets, e.g., the read-only Notifications feed (kind "Info") live in the same
// "Personal" category as user-created, sendable personal channels (kind "Standard").

// Sidebar grouping only. "World" leads (News lives under it); "Role" is a stronger world
// channel; "Personal" collects the per-viewer Notifications feed and user-made personal
// channels. Categories hold no significance beyond display.
export type ChannelCategory =
  | "Raw" | "Lounge" | "Groupchat" | "Notebook" | "Role"
  | "World" | "Org" | "Prosecution" | "Bug" | "Personal";
export const CHANNEL_CATEGORIES: ChannelCategory[] = [
  "Raw", "Lounge", "Groupchat", "Notebook", "Role",
  "World", "Org", "Prosecution", "Bug", "Personal",
];

// Behavioural type: inherent properties a channel carries regardless of category. Only
// properties that can't be derived elsewhere live here — a channel that merely has an
// associated object (a notebook, a group chat) stays "Standard" and is recognised via its
// mapping (notebook_for_channel / gc_key_for_channel), not a dedicated kind.
//   - "Standard": an ordinary channel; sendability follows engine perms.
//   - "Info":     a frontend-only, read-only feed (name reveals, bug alerts). Not engine-backed;
//                 lives per-view in GameView.info_channels. Always readable, never sendable.
//   - "Bug":      a surveillance feed of a bug's relayed messages. Read-only, non-interactable;
//                 held globally (game.bugs, "bug:*"), shown per GameView.visible_bugs.
export type ChannelKind = "Standard" | "Info" | "Bug";


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

// Entries that render inside a read-only Info channel (as opposed to WorldEvents,
// which render in News). Kept separate so info-channel content never leaks into the
// world-event/news stream.
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
  // The viewer was told they've been bugged (directed Bugged command). A personal event, so
  // it lands in the personal Notifications info channel — NOT News (which is world events).
  // context says why (explicit ability vs custody); who planted it is intentionally unknown.
  Bugged: {
    context: BugContext,
  }
} | {
  // The viewer's own role was (re)assigned — "your role is now X" in their notifications log.
  RoleUpdate: {
    role: Role,
  }
} | {
  // The viewer's own true name was set — "your true name is now X".
  TrueNameUpdate: {
    true_name: string,
  }
} | {
  // The viewer received a notebook (any source — pass, gift, role grant). Derived on the
  // frontend from gaining read access to a notebook channel; no engine command backs it.
  NotebookReceived: Record<string, never>,
}

// A poll started (outcome null) or ended (outcome set), rendered inline in the poll's
// scoped channel/news feed. Distinct from the Polls panel, which is where you actually vote.
export type PollNoticeEvent = {
  PollNotice: {
    poll_id: string,
    subject: PollSubject,
    outcome: PollOutcome | null,
    // Actor KEY of who opened the vote (null = none, and always null on a close notice). Resolved
    // for display at render time — see PollData.opener for why it is not a name.
    opener: string | null,
  }
}

export type GameEvent = {
  timestamp: number,
  data: { Message: Message } | { Write: WriteEvent } | WorldEvent | InfoEvent | PollNoticeEvent,
}

// Shared, globally-held poll data (subject + scope + tally). Mirrors the engine's
// UpdatePoll command. Visible-to-whom is decided per viewer by poll_views below.
export type PollData = {
  subject: PollSubject,
  scope: PollVisibility,
  accept: number,
  reject: number,
  potential: number,
  // Actor KEY of whoever opened the vote (null = none), not a name. Resolved for display at
  // render time via actor_name: a name resolved at apply time would be whatever `players` happened
  // to hold then, and a view replaying this later would resolve it differently.
  opener: string | null,
  // Set once the poll has resolved. The entry is KEPT rather than deleted — client state is
  // monotonic, and a view that gains the poll's viewport later replays its whole history and has to
  // reach the same place. Consumers showing live polls filter on this being null.
  outcome: PollOutcome | null,
}

// A viewer's personal relationship to a poll (from the directed UpdatePollView). Having
// an entry at all means the viewer can see the poll; `eligible` is whether they may vote.
export type PollView = {
  eligible: boolean,
  own_vote: boolean | null,
}

// Shared, globally-held prosecution snapshot (from the broadcast UpdateProsecution). The trial
// channel and verdict poll ride their own command streams; trial_channel is just the id so the
// UI can tag that channel as a prosecution channel. Per-viewer "am I frozen" lives on GameView.
export type ProsecutionData = {
  prosecutor_display: ActorDisplay,
  defendant_display: ActorDisplay,
  // Set once the defendant picks one. Public — a trial's defence counsel is not a hidden fact.
  lawyer_display: ActorDisplay | null,
  phase: ProsecutionPhaseView,
  trial_channel: string | null,
  // The viewport this snapshot arrived through, or null for the admin mirror. The engine no
  // longer sends a "you are viewing frozen state" notice — it doesn't need to, because the fact
  // is derivable: if this view no longer holds the viewport that delivered the update, no more
  // are coming and what's displayed is the last thing it heard. See GameView.prosecution_frozen.
  viewport: string | null,
}

export interface AbilityView {
  name: AbilityName;
  // Split by outcome: conditional charge subtraction means successful and failed uses
  // can have different remaining counts (e.g. a true-name guess bounded by an attempts
  // pool on failure but also an invite pool on success).
  success_usages_remaining: number;
  failure_usages_remaining: number;
  iterations_to_reset: number;
}

// A passive a viewer holds. No charges/usages, but the type itself may carry data (e.g.
// VoteAmplification's multiplier), so this is the full PassiveType. Observable-only,
// rendered in the Passives panel beside abilities.
export interface PassiveView {
  type: PassiveType;
}

// A single member of a channel, as seen by one viewer. Membership is per-view
// because the same actor can be shown under different displays to different viewers
// (deception). Keyed by display key -> member.
export type ChannelMemberView = {
  display: ActorDisplay;
  perms: number;
  // Sticky: set once this member has ever held a positive permission. A member that
  // never had one isn't an effective member and shouldn't be shown as one.
  had_positive: boolean;
};

// Everything a viewer knows about a channel they're a member of: their permissions
// and the members they can see. The presence of an entry (hence perms) IS the
// membership signal — non-members hold no entry and receive no member updates.
export type ChannelView = {
  perms: ChannelPerms;
  members: SvelteMap<string, ChannelMemberView>;
  // The displays this viewer may send as in this channel (their "send as" options).
  displays: ActorDisplay[];
};

export class GameView {
  // channel key -> the viewer's perms + visible members. Only channels the viewer is
  // (or was) a member of appear here.
  channel_views = new SvelteMap<string, ChannelView>();
  events: GameEvent[] = $state([]); // should only store world events
  // Frontend-only, read-only "info" channels private to this viewer (name reveals,
  // autopsies, …). Keyed by a synthetic "info:*" key; not engine channels.
  info_channels = new SvelteMap<string, Channel>();
  abilities = new SvelteMap<string, AbilityView>();
  // passive id -> the passive this viewer holds (from UpdatePassiveView). Observable list.
  passives = new SvelteMap<string, PassiveView>();
  // gc keys this viewer owns (from GcOwnerStatus). Drives the group-chat controls.
  owned_gcs = new SvelteSet<string>();
  // poll id -> this viewer's personal view of a poll they can see (from UpdatePollView).
  // The shared poll data lives in GameState.polls; this is just eligibility + own vote.
  poll_views = new SvelteMap<string, PollView>();
  // prosecution id -> this viewer's latest received snapshot (from UpdateProsecution). Per-view
  // rather than global so that each view diffs the stream IT receives: a phase change vs the
  // stored entry is what emits a news event, and an absent player replaying deferred updates in
  // order reproduces the same ordered news timeline. Drives both the Prosecutions panel and news.
  prosecutions = new SvelteMap<string, ProsecutionData>();
  // bug channel keys ("bug:*") this viewer has been given access to. The bug logs themselves are
  // global (GameState.bugs); this is the per-viewer gate. Never shrinks: losing access to a bug
  // stops the relay, it does not unsee what was already relayed.
  visible_bugs = new SvelteSet<string>();
  // viewport keys this view currently has access to, tracked from EnterViewport/ExitViewport.
  // This is what routes a Viewport-addressed command to the right views when a client holds
  // several actors — the command names a viewport, and every view holding it receives it.
  //
  // Unlike visible_bugs this DOES shrink on exit: it answers "is more content still coming",
  // not "have I ever seen any".
  viewports = new SvelteSet<string>();

  // viewport key -> the log position, exclusive, up to which THIS view has been given that
  // viewport's commands. Monotonic; every write goes through deliver_to.
  //
  // Per-VIEW, which is the whole point. The server's equivalent is per-connection, so a connection
  // holding several actors is sent a viewport's history exactly once no matter how many of its
  // actors enter. This is what lets the second one be handed its own copy. See GameState.#backfill.
  #watermark = new Map<string, number>();

  delivered(viewport: string): number {
    return this.#watermark.get(viewport) ?? 0;
  }

  deliver_to(viewport: string, position: number) {
    this.#watermark.set(viewport, Math.max(this.delivered(viewport), position));
  }

  // Is state that arrived through this viewport still live?
  //
  // A viewport this view no longer holds will deliver nothing further, so everything it did
  // deliver is the last thing heard rather than the current state. That is the whole meaning of
  // leaving a viewport, and it is not specific to any one kind of state — a channel, an org, a
  // bug feed and a prosecution all go stale the same way and for the same reason.
  //
  // Nothing is retracted: what was received stays. This only says it has stopped moving, which is
  // the difference between showing someone what they knew and lying to them about what is.
  frozen(viewport: string | null | undefined): boolean {
    return viewport != null && !this.viewports.has(viewport);
  }

  // The prosecution case, which stores its own source viewport on the snapshot. This is what the
  // deleted FreezeProsecutionView command used to say out loud.
  prosecution_frozen(id: string): boolean {
    return this.frozen(this.prosecutions.get(id)?.viewport);
  }

}

// Every command this client has received, in the order it received them, plus the index over the
// viewport-addressed ones.
//
// Deliberately the same structure as yagami's History: the server holds exactly this and filters it
// per connection, and the client's problem is the same one at a finer granularity. Not reactive —
// nothing renders from the log itself, only from the state applying it produces.
class History {
  #log: CommandPayload[] = [];
  // viewport key -> its positions in #log, ascending. Positions, not payloads: there is exactly one
  // copy of every command.
  #index = new Map<string, number[]>();

  // Record one received command and return its position, which is what watermarks are measured in.
  append(payload: CommandPayload): number {
    const pos = this.#log.length;
    this.#log.push(payload);

    const viewport = recipientToViewport(payload.recipient);
    if (viewport !== undefined) {
      let positions = this.#index.get(viewport);
      if (!positions) {
        positions = [];
        this.#index.set(viewport, positions);
      }
      positions.push(pos);
    }

    return pos;
  }

  // Everything addressed to `viewport` in [from, until), with each command's position. Walks that
  // viewport's positions rather than the log, so it costs what it yields.
  //
  // Note what CANNOT come back from here: EnterViewport and ExitViewport are addressed to the actor
  // they concern, never to a viewport, so a replay of a viewport's history can never contain another
  // access change. That is what keeps #backfill from recursing.
  *range(viewport: string, from: number, until: number): Generator<[number, CommandPayload]> {
    for (const pos of this.#index.get(viewport) ?? []) {
      if (pos < from) continue;
      if (pos >= until) return;
      yield [pos, this.#log[pos]];
    }
  }
}

// Whether two prosecution phase-views are the same. Subphases collapse into the view already
// (Grace/Presentation both read as e.g. Trial:Prosecutor), so this is what "the phase changed".
// Whether two snapshots are the same PHASE, ignoring the ready/done flags inside it. Mirrors
// ProsecutionPhaseView::same_phase in the engine.
//
// Signalling ready changes the snapshot without changing the phase, so comparing whole values
// would announce the prosecution again on every signal. The subphase IS compared — grace →
// presentation is a real transition.
export function phaseViewEqual(a: ProsecutionPhaseView, b: ProsecutionPhaseView): boolean {
  if (a === "Voting" || b === "Voting") return a === b;
  if ("Custody" in a || "Custody" in b) return "Custody" in a && "Custody" in b;

  const [x, y] = [a.Trial, b.Trial];
  if ("Debate" in x || "Debate" in y) return "Debate" in x && "Debate" in y;
  if ("Prosecutor" in x && "Prosecutor" in y) return x.Prosecutor === y.Prosecutor;
  if ("Defense" in x && "Defense" in y) return x.Defense === y.Defense;
  return false;
}

// The side holding the floor, or null outside the trial. For rendering "whose turn" without
// destructuring the nested union at every call site.
export function trialFloor(phase: ProsecutionPhaseView): "Prosecutor" | "Defense" | "Debate" | null {
  if (phase === "Voting" || "Custody" in phase) return null;
  if ("Prosecutor" in phase.Trial) return "Prosecutor";
  if ("Defense" in phase.Trial) return "Defense";
  return "Debate";
}

// A short label for the Prosecutions panel.
export function phaseLabel(phase: ProsecutionPhaseView): string {
  if (phase === "Voting") return "Verdict vote";
  if ("Custody" in phase) return "In custody";
  if ("Debate" in phase.Trial) return "Trial · debate";
  const grace = ("Prosecutor" in phase.Trial ? phase.Trial.Prosecutor : phase.Trial.Defense) === "Grace";
  const side = "Prosecutor" in phase.Trial ? "prosecution" : "defense";
  return grace ? `Trial · ${side} to begin` : `Trial · ${side} speaking`;
}

// The sentence announcing a prosecution reaching this phase, for news feeds and toasts.
// Both of those render the same event, so they share the wording rather than drifting apart.
export function phaseAnnouncement(
  phase: ProsecutionPhaseView,
  prosecutor: string,
  defendant: string,
  ended: boolean,
): string {
  if (ended) return `The prosecution of ${defendant} has ended.`;
  if (phase === "Voting") return `The verdict vote for ${defendant} has begun.`;
  if ("Custody" in phase) return `${prosecutor} is prosecuting ${defendant}.`;
  if ("Debate" in phase.Trial) return `The trial of ${defendant} has entered debate.`;

  if ("Prosecutor" in phase.Trial) {
    return phase.Trial.Prosecutor === "Grace"
      ? `The trial of ${defendant} has begun — the prosecution has the floor.`
      : `In the trial of ${defendant}, the prosecution presents.`;
  }
  return phase.Trial.Defense === "Grace"
    ? `In the trial of ${defendant}, the defense has the floor.`
    : `In the trial of ${defendant}, the defense presents.`;
}

// Stable map key for an ActorDisplay (the tagged union isn't usable as a key directly).
export function displayKey(d: ActorDisplay): string {
  if (typeof d === "string") return d; // "Mysterious" | "System"
  if ("Raw" in d) return `Raw:${slotKeyToString(d.Raw)}`;
  if ("Org" in d) return `Org:${slotKeyToString(d.Org)}`;
  return `Role:${d.Role}`;
}

// Human-readable label for an ActorDisplay. Canonical home for what ChannelView and
// Prosecutions each still open-code as a local `display_string`; needs the players map to
// resolve a Raw key to its name, so it takes one rather than closing over game state.
export function actorLabel(d: ActorDisplay, players: ReadonlyMap<string, Player>): string {
  if (d === "Mysterious") return "???";
  if (d === "System") return "System";
  if ("Raw" in d) return playerLabel(slotKeyToString(d.Raw), players);
  if ("Role" in d) return d.Role;
  if ("Org" in d) return "Org";
  return "Unknown";
}

// Channel permission bits, mirroring ChannelPermission in the engine.
export const PERM_SEND = 1;
export const PERM_VIEW = 2;
export const PERM_LOGGABILITY = 4;

// Whether a perms value grants at least one permission. Used to derive the sticky
// had_positive flag — a membership is "effective" if it has EVER been positive.
export function hasPositivePerms(perms: number): boolean {
  return (perms & (PERM_SEND | PERM_VIEW | PERM_LOGGABILITY)) !== 0;
}

// A player slot this client has been told about (from MapPlayer).
//
// The slot comes first and always; the NAME is a separate server-level fact that may never arrive.
// So `display_name` is nullable, and every render site goes through `playerLabel` rather than
// reaching for the field — an unnamed slot is a normal, permanent state, not a missing value.
export interface Player {
  display_name: string | null;
}

// What to call a player slot. Falls back to a generated label rather than a bare key, matching how
// every other unnamed object in the UI reads ("lounge-3", "trial-1v0").
export function playerLabel(id: string, players: ReadonlyMap<string, Player>): string {
  const name = players.get(id)?.display_name;
  if (name) return name;
  const key = slotKeyFromString(id);
  return `player-${key.idx}v${key.version}`;
}

// Admin-facing per-player facts, populated from the System copy of the personal-info commands
// (RoleUpdate / TrueNameUpdate). In the real server only the host receives the System copy, so
// this stays empty on ordinary player clients. Surfaced when admin inspects a player.
export interface PlayerInfo {
  role?: Role;
  true_name?: string;
}

export function new_player(display_name: string | null): Player {
  let player: Player = $state({ display_name });
  return player;
}

// A first-class org actor, held globally (like channels). Members and abilities arrive
// on directed Actor(org) commands but are stored globally; a viewer is shown the org and
// its contents iff they're a member (all members see the full list). `abilities` mirrors
// the org's UpdateAbilityView stream, keyed by ability id.
export interface Org {
  name: OrganizationName;
  channel_id: string; // backing channel key
  members: SvelteSet<string>; // org member player ids (dead members included)
  abilities: SvelteMap<string, AbilityView>;
}

export function new_org(name: OrganizationName, channel_id: string): Org {
  return {
    name,
    channel_id,
    members: new SvelteSet<string>(),
    abilities: new SvelteMap<string, AbilityView>(),
  };
}

// Human-readable org names for the terse config codes.
const ORG_NAMES: Record<OrganizationName, string> = {
  NULL: "Null",
  KK: "Kira's Kingdom",
  TF: "Task Force",
  SPK: "SPK",
};

export function orgDisplayName(name: OrganizationName): string {
  return ORG_NAMES[name] ?? name;
}

// Channels must be $state proxies, not plain objects: SvelteMap only tracks its
// own get/set, not deep mutations to stored values. Without this, channel.events.push
// and channel.archived = true don't trigger reactivity, so views go stale until
// something else (e.g. switching channels) forces a recompute.
export function new_channel(kind: ChannelKind, category: ChannelCategory, name: string): Channel {
  let channel: Channel = $state({ kind, category, name, archived: false, events: [] });
  return channel;
}

// Create or refresh one entry in an ability list. Shared because the same UpdateAbilityView lands
// either in a player's own list or in an org's shared one, depending only on how it was addressed.
// Takes the command's fields directly: the engine's variant is an inline struct, so bindings has
// no name for it.
function upsert_ability(
  abilities: SvelteMap<string, AbilityView>,
  { ability_id, ability_name, success_usages_remaining, failure_usages_remaining, iterations_to_reset }: {
    ability_id: AbilityKey;
    ability_name: AbilityName;
    success_usages_remaining: number;
    failure_usages_remaining: number;
    iterations_to_reset: number;
  },
) {
  const id = slotKeyToString(ability_id);
  const existing = abilities.get(id);
  if (existing) {
    existing.success_usages_remaining = success_usages_remaining;
    existing.failure_usages_remaining = failure_usages_remaining;
    existing.iterations_to_reset = iterations_to_reset;
    return;
  }
  const view: AbilityView = $state({
    name: ability_name,
    success_usages_remaining,
    failure_usages_remaining,
    iterations_to_reset,
  });
  abilities.set(id, view);
}

// A kidnapping this client has been told about. Kept after its reveal rather than deleted: the
// reveal command carries only the kidnapping's id, so resolving the victim is a lookup here, and a
// view replaying that reveal later must resolve the same one.
export type TrackedKidnapping = {
  victim: string;
  duration: number | null; // null = indefinite (no scheduled auto-release)
  revealed: boolean;
};

// The same shape for an incarceration, and kept for the same reason: the release command carries
// only the id, so the victim is a lookup here.
export type TrackedIncarceration = {
  victim: string;
  duration: number | null; // null = held until released
  released: boolean;
};

// Synthetic key of the single, per-viewer read-only Notifications info channel. It is the one
// place directed-at-you personal events land: reveal results (true names, notebook holdings)
// and bug alerts ("you've been bugged"). NOT News (world events), and NOT a "personal channel"
// (that's a real engine channel). Add new personal event kinds here, don't spawn a second one.
export const NOTIF_CHANNEL = "info:notifs";

// Namespaced channel key for a bug's surveillance feed. Bugs live in their own BugKey slot
// space, which can collide with real ChannelKeys, so the "bug:" prefix keeps them separate
// in the (channel-keyed) resolve path.
export function bugChannelKey(bug_id: BugKey): string {
  return `bug:${slotKeyToString(bug_id)}`;
}

// Maps a command recipient to the key of the single view it targets. Actor recipients key by
// their slot; System is the admin view (world events mirrored for admin land there). Returns
// undefined for a Viewport recipient, which does not name one view — use recipientToViews.
export function recipientToView(rec: CommandRecipient): string | undefined {
  if (rec === "System") return "System";
  if (typeof rec !== "string" && "Actor" in rec) return slotKeyToString(rec.Actor);
  return undefined;
}

export function recipientToViewport(rec: CommandRecipient): string | undefined {
  if (typeof rec !== "string" && "Viewport" in rec) return slotKeyToString(rec.Viewport);
  return undefined;
}

function recipientToPlayer(recipient: CommandRecipient): string | undefined {
  if (typeof recipient !== "string" && "Actor" in recipient) {
    return slotKeyToString(recipient.Actor);
  }
}

export class GameState {
  #channel_to_notebook = new SvelteMap<string, string>();
  #notebook_to_channel = new SvelteMap<string, string>();
  #channel_to_gc = new SvelteMap<string, string>();
  #channel_to_org = new SvelteMap<string, string>();
  // trial channel key -> prosecution id. Not for rendering (that's driven by the "Prosecution"
  // channel category); this is so an action taken from within the channel can find its prosecution.
  #channel_to_prosecution = new SvelteMap<string, string>();
  // channel key -> whether the channel is currently loggable (a global channel property from
  // SetChannelLoggable). Kept separate from the Channel object so it lands regardless of
  // whether the establishing Map* command has arrived yet.
  #channel_loggable = new SvelteMap<string, boolean>();
  // notebook key -> whether it's currently on loan (a global notebook property from
  // NotebookBorrowingStatus). Shown in the notebook channel.
  #notebook_borrowed = new SvelteMap<string, boolean>();
  // viewport key -> the bug channel key ("bug:*") whose content rides it. Learned from NewBug,
  // which is the first thing addressed to a bug's viewport, so it always arrives in the backfill
  // that follows an EnterViewport. Bugs are the one object whose per-view gate has no Actor-
  // addressed carrier of its own, so this is the only place the association is needed.
  #viewport_to_bug = new SvelteMap<string, string>();
  // viewport key -> the org whose backing channel it is. Learned from MapOrg, which is the
  // first thing addressed to an org channel's viewport. Needed because an org's abilities are
  // addressed there rather than to the org actor — "everyone in the org" is now expressed as
  // "everyone who can see the org's channel".
  #viewport_to_org = new SvelteMap<string, string>();
  // channel key -> the viewport its content rides. Every Map* command is addressed to exactly that
  // viewport, so registering a channel is where this is learned; see #map_channel.
  //
  // What it answers is "has this channel gone quiet for me" — a view that no longer holds the
  // viewport will receive nothing further about the channel, so what it has is a snapshot. See
  // GameView.frozen.
  #channel_to_viewport = new SvelteMap<string, string>();
  // The real News channel's key once one exists (set in MapWorldChannel). $state so
  // that views resolving news's backing channel recompute the moment it's assigned —
  // otherwise selecting news before the channel exists never picks up its perms.
  // Left pointing at a stale key after removal so news falls back to event-log-only.
  news_channel_id = $state<string | null>(null);
  channels = new SvelteMap<string, Channel>();
  players = new SvelteMap<string, Player>();
  // admin-facing per-player facts (role, true name) from the System copy of personal-info
  // commands. Keyed by player id. Only populated on the admin/host client; read when admin
  // inspects a player.
  player_info = new SvelteMap<string, PlayerInfo>();
  views = new SvelteMap<string, GameView>();
  // poll id -> shared poll data (subject, scope, tally). Held globally like channels;
  // per-viewer visibility is decided by each view's poll_views entries.
  polls = new SvelteMap<string, PollData>();
  // org id -> org (name, backing channel, members, abilities). Held globally; a viewer
  // is shown an org iff they're a member. See Org.
  orgs = new SvelteMap<string, Org>();
  // bug channel key ("bug:*") -> the bug's surveillance feed (a read-only Channel of the
  // relayed messages). Held globally like channels; per-viewer visibility is each view's
  // visible_bugs, opened by EnterViewport on the bug's viewport.
  bugs = new SvelteMap<string, Channel>();
  // kidnapping id -> tracked kidnapping. Populated by the Kidnapping world event; the KidnapReveal
  // event references the id to resolve the victim, and this is the hook for a future live-countdown
  // timer. Held globally like bugs. See TrackedKidnapping for why entries outlive their reveal.
  kidnappings = new SvelteMap<string, TrackedKidnapping>();
  // incarceration id -> tracked incarceration. Same role as `kidnappings`: the release command
  // carries only the id, so the victim is resolved here.
  incarcerations = new SvelteMap<string, TrackedIncarceration>();

  constructor() {
    // System (admin) is not a player: it bypasses channel perms (see is_admin), so its
    // perms/abilities stay empty. It exists to hold the state authority can't cover, like the
    // world-event mirror. There is no longer a "Base" view — it existed to seed new players
    // with the BasePlayer catch-up stream, and viewport backfill replaced that entirely.
    this.views.set("System", new GameView());
  }

  // Every command received, in order. The source the per-view state below is built from, and what
  // lets a view that gains a viewport be handed that viewport's past. See #backfill.
  #history = new History();

  // Apply a batch of commands in push order. The public seam the Sequencer drives;
  // command ordering within a batch is significant (create-before-reference,
  // last-write-wins perms), so never reorder.
  apply_batch(commands: CommandPayload[]) {
    for (const payload of commands) {
      this.#apply(payload);
    }
  }

  // Apply one command: record it, run its global effect once, then its per-view effect for each of
  // this client's views that receives it.
  //
  // The split is the rule for where state lives, rather than a comment on each command. Anything in
  // #apply_global happens once per command and is shared by every view; anything in #apply_to_view
  // happens once per receiving view and may be REPLAYED later for a view that gains the viewport
  // afterwards. Which function a new command goes in IS the decision.
  //
  // The consequence of replay is that a per-view handler may not destroy anything: it can be run
  // long after the fact, and whatever it reads must still be there. That is why a resolved poll and
  // a revealed kidnapping are marked rather than deleted.
  #apply(payload: CommandPayload) {
    const pos = this.#history.append(payload);
    this.#apply_global(payload);
    for (const view of this.#recipients(payload.recipient)) {
      this.#apply_to_view(view, payload, pos);
    }
  }

  // Which of this client's views a command lands in.
  //
  // The server already decided this CONNECTION may see it; this decides which of the views the
  // connection holds. An Actor-addressed command names exactly one. A Viewport-addressed one names
  // none — the server sent it because SOME actor here has access, and which ones is a question only
  // the client can answer. Fanning out here is what lets one client hold several actors without the
  // protocol having to know.
  #recipients(recipient: CommandRecipient): GameView[] {
    const viewport = recipientToViewport(recipient);
    if (viewport === undefined) {
      const view = this.actor_view(recipient);
      return view ? [view] : [];
    }

    const out: GameView[] = [];
    for (const [key, view] of this.views) {
      // System reads every viewport — the server sends it everything, and it holds no actors of its
      // own to enter with. That is what makes admin able to watch a deception: they see the fiction
      // through the same viewport the players do, and any truth the engine exposes arrives
      // separately as a System-addressed command to compose against it.
      if (key === "System" || view.viewports.has(viewport)) out.push(view);
    }
    return out;
  }

  // Hand one view the part of a viewport's past it has not been given.
  //
  // This exists because the server backfills a viewport once per CONNECTION — only its first holder
  // — which is correct for a connection and insufficient here, where state is per-actor. A key
  // holding several actors (an admin key holds every actor) has already been sent viewports that a
  // view entering now never saw, and no second backfill is coming.
  //
  // The two backfills are complementary and cannot overlap: the server sends what the connection
  // lacked, this replays what the connection had and this view lacked. The watermark is what
  // separates them, and it would suppress a double anyway.
  #backfill(view: GameView, viewport: string, until: number) {
    for (const [pos, payload] of this.#history.range(viewport, view.delivered(viewport), until)) {
      this.#apply_to_view(view, payload, pos);
    }
    view.deliver_to(viewport, until);
  }

  // The view for an actor, created on demand.
  //
  // Views cannot wait for AddPlayer's *response* to create them: an action's commands are
  // applied before its response is settled, and a player's own creation batch is already full
  // of commands addressed to them (UpdateChannelView, EnterViewport, RoleUpdate). Creating the
  // view lazily on first sight is what makes that batch land instead of being dropped — or,
  // in UpdateChannelView's case, throwing and taking the whole batch with it.
  //
  // Actors that are not players never reach here, because nothing is addressed to an org actor
  // any more (see owner_view_recipient in the engine).
  private view_for(key: string): GameView {
    let view = this.views.get(key);
    if (!view) {
      view = new GameView();
      this.views.set(key, view);
    }
    return view;
  }

  // Does the named view receive a command addressed this way? The public form of #recipients,
  // for callers that already know which view they care about (e.g. deciding whether to toast for
  // the one the user is looking at).
  view_receives(recipient: CommandRecipient, view_key: string): boolean {
    const viewport = recipientToViewport(recipient);
    if (viewport !== undefined) {
      if (view_key === "System") return true; // see #recipients
      return this.views.get(view_key)?.viewports.has(viewport) ?? false;
    }
    return recipientToView(recipient) === view_key;
  }

  // The single view an Actor-addressed command lands in, created if this is the first thing
  // we've seen for that actor. Returns undefined only for a recipient that names no single
  // view (System when it has somehow gone missing, or a Viewport — use #recipients for those).
  private actor_view(recipient: CommandRecipient): GameView | undefined {
    const key = recipientToView(recipient);
    return key === undefined ? undefined : this.view_for(key);
  }

  // Register a channel, recording the viewport it arrived on.
  //
  // One place, because every Map* does exactly this and the viewport half is easy to forget: a
  // channel registered without it can never answer whether it has gone quiet.
  #map_channel(recipient: CommandRecipient, key: string, channel: Channel) {
    this.channels.set(key, channel);
    const viewport = recipientToViewport(recipient);
    if (viewport !== undefined) this.#channel_to_viewport.set(key, viewport);
  }

  // What a command does to state every view shares. Runs exactly once, and is never replayed —
  // so unlike a per-view handler this one may consume what it reads.
  #apply_global({ recipient, cmd, timestamp }: CommandPayload) {
    if ("MapLounge" in cmd) {
      const { channel_id, contact_id } = cmd.MapLounge;
      // Lounges are identified by their contact-channel id.
      const name = `lounge-${contact_id}`;

      this.#map_channel(recipient, slotKeyToString(channel_id), new_channel("Standard", "Lounge", name));

      return;
    }

    if ("MapGc" in cmd) {
      const { gc_id, channel_id, contact_id } = cmd.MapGc;
      const channel_key = slotKeyToString(channel_id);
      // Same shape as lounges for now (custom gc names arrive with the server).
      const name = `groupchat-${contact_id}`;

      this.#map_channel(recipient, channel_key, new_channel("Standard", "Groupchat", name));
      this.#channel_to_gc.set(channel_key, slotKeyToString(gc_id));

      return;
    }

    // A player slot exists. The engine says nothing about who is on it — that arrives on the
    // profile channel, possibly never, so the entry starts unnamed and renders as `player-<slot>`.
    //
    // Created here rather than from an action response, which is what makes the roster
    // reconstructible: the response reaches only the connection that submitted AddPlayer, so any
    // other client (and the submitter after a rejoin) used to have an empty players map.
    if ("MapPlayer" in cmd) {
      const key = slotKeyToString(cmd.MapPlayer.player_id);
      if (!this.players.has(key)) this.players.set(key, new_player(null));
      return;
    }

    // Register an org and its backing channel (an "Org"-kind channel, mirroring MapGc).
    if ("MapOrg" in cmd) {
      const { org_id, channel_id, org_name } = cmd.MapOrg;
      const channel_key = slotKeyToString(channel_id);
      const org_key = slotKeyToString(org_id);

      this.#map_channel(recipient, channel_key, new_channel("Standard", "Org", orgDisplayName(org_name)));
      this.#channel_to_org.set(channel_key, org_key);
      this.orgs.set(org_key, new_org(org_name, channel_key));
      // MapOrg is the first thing addressed to the org channel's viewport, so this is where
      // that viewport becomes identifiable — the org's abilities arrive through it afterwards.
      const viewport = recipientToViewport(recipient);
      if (viewport !== undefined) this.#viewport_to_org.set(viewport, org_key);
      return;
    }

    // Org membership (global, all members see the full list). The command carries the
    // org id directly, so no recipient mapping is needed.
    if ("AddOrgMember" in cmd) {
      const { player_id, org_id } = cmd.AddOrgMember;
      this.orgs.get(slotKeyToString(org_id))?.members.add(slotKeyToString(player_id));
      return;
    }

    if ("RemoveOrgMember" in cmd) {
      const { player_id, org_id } = cmd.RemoveOrgMember;
      this.orgs.get(slotKeyToString(org_id))?.members.delete(slotKeyToString(player_id));
      return;
    }

    if ("MapNotebook" in cmd) {
      const { notebook_id, channel_id } = cmd.MapNotebook;
      const channel_key = slotKeyToString(channel_id);
      const notebook_key = slotKeyToString(notebook_id);

      this.#map_channel(
        recipient,
        channel_key,
        new_channel("Standard", "Notebook", "Death Notebook" + '-' + notebook_id.idx + 'v' + notebook_id.version),
      );
      this.#channel_to_notebook.set(channel_key, notebook_key);
      this.#notebook_to_channel.set(notebook_key, channel_key);

      return;
    }

    if ("MapWorldChannel" in cmd) {
      const { channel_name, channel_id } = cmd.MapWorldChannel;

      // World channels are ordinary (sendable-per-perms) channels; only their sidebar
      // grouping varies — L & Watari sits under Roles, the rest under World.
      let category: ChannelCategory = "World";
      if (channel_name == "LAndWatari") {
        category = "Role";
      }

      const key = slotKeyToString(channel_id);
      // News is special: it must always appear to exist even after the underlying
      // channel is removed (world events render into it regardless of the channel's
      // existence or the viewer's perms). Remember its id so the UI can find it.
      if (channel_name === "News") {
        this.news_channel_id = key;
      }

      this.#map_channel(recipient, key, new_channel("Standard", category, channel_name));

      return;
    }

    // Register a personal channel: a real, sendable channel private to its owner. Global like
    // the other channel maps; per-viewer visibility falls out of channel-view perms (only the
    // owner is a member). Kind "Personal" so the sidebar groups it with the info feeds.
    if ("MapPersonalChannel" in cmd) {
      const { channel_id } = cmd.MapPersonalChannel;
      const key = slotKeyToString(channel_id);
      if (!this.channels.has(key)) {
        this.#map_channel(recipient, key, new_channel("Standard", "Personal", `personal-${channel_id.idx}v${channel_id.version}`));
      }
      return;
    }

    // can only be directed to system
    if ("ArchiveChannel" in cmd) {
      const channel_id = slotKeyToString(cmd.ArchiveChannel.channel_id);
      const ch = this.channels.get(channel_id);
      if (ch) ch.archived = true;
      return;
    }

    // A channel's global loggability (initial value on creation, then on each toggle).
    if ("SetChannelLoggable" in cmd) {
      const { channel_id, loggable } = cmd.SetChannelLoggable;
      this.#channel_loggable.set(slotKeyToString(channel_id), loggable);
      return;
    }

    // Whether a notebook is currently on loan (emitted on each possession change).
    if ("NotebookBorrowingStatus" in cmd) {
      const { notebook_id, borrowed } = cmd.NotebookBorrowingStatus;
      this.#notebook_borrowed.set(slotKeyToString(notebook_id), borrowed);
      return;
    }

    if ("AddMessage" in cmd) {
      const { channel_id, content, sender_display } = cmd.AddMessage;
      const key = slotKeyToString(channel_id);
      // its not possible to add a message to a channel that doesnt exist
      const channel = this.channels.get(key)!;
      channel.events.push({
        timestamp,
        data: {
          Message: {
            content,
            sender_display,
          }
        },
      });
      return;
    }

    // writes are treated the exact same as messages, so they should be stored using the same mechanism 
    if ("NotebookWrite" in cmd) {
      const { notebook_id, user_id, message, true_name, delay, successes_remaining, attempts_remaining, success, target_saved } = cmd.NotebookWrite;
      const channel_key = this.#notebook_to_channel.get(slotKeyToString(notebook_id));
      if (channel_key) {
        const channel = this.channels.get(channel_key);
        if (channel) {
          channel.events.push({
            timestamp,
            data: {
              Write: {
                user_id: slotKeyToString(user_id),
                notebook_id: slotKeyToString(notebook_id),
                message: message ?? "",
                target_saved,
                success,
                successes_remaining,
                attempts_remaining,
                delay,
                true_name,
              }
            }
          });
        }
      }
      return;
    }

    // An ORG's abilities are held globally — every member sees the same list — and are addressed
    // to the org channel's viewport rather than to the org actor. A player's own abilities are
    // Actor-addressed and per-view; see #apply_to_view.
    if ("UpdateAbilityView" in cmd) {
      const abilities = this.#org_abilities(recipient);
      if (abilities) upsert_ability(abilities, cmd.UpdateAbilityView);
      return;
    }

    if ("RemoveAbility" in cmd) {
      this.#org_abilities(recipient)?.delete(slotKeyToString(cmd.RemoveAbility.ability_id));
      return;
    }

    // Shared poll data (global). First sighting drops a "started" notice into the poll's
    // scoped channel; later UpdatePolls just refresh the tally.
    if ("UpdatePoll" in cmd) {
      const { poll_id, subject, scope, accept, reject, potential, opener } = cmd.UpdatePoll;
      const key = slotKeyToString(poll_id);
      const opener_key = opener ? slotKeyToString(opener) : null;
      const existing = this.polls.get(key);
      // First sight = the vote just opened; drop the "vote started" notice into its scoped
      // channel (later UpdatePolls are tally refreshes).
      if (!existing) {
        this.poll_notice(scope, key, subject, null, timestamp, opener_key);
      }
      this.polls.set(key, {
        subject, scope, accept, reject, potential,
        opener: opener_key,
        // an update after the close would be odd, but it must not un-resolve the poll.
        outcome: existing?.outcome ?? null,
      });
      return;
    }

    // A poll concluded: record the outcome and drop a resolution notice into its scoped channel.
    //
    // The entry is kept, not deleted. A view gaining the poll's viewport later replays this whole
    // history and must reach the same place, and the notice below reads the poll's own subject.
    // Consumers showing live polls filter on `outcome`; the per-view poll_views entries are left
    // alone for the same reason (nothing received is retracted).
    if ("ClosePoll" in cmd) {
      const { poll_id, outcome } = cmd.ClosePoll;
      const key = slotKeyToString(poll_id);
      const poll = this.polls.get(key);
      if (poll) {
        // A resolution notice has no opener (it's the outcome, not the opening).
        this.poll_notice(poll.scope, key, poll.subject, outcome, timestamp, null);
        // re-set rather than mutate: SvelteMap tracks its own get/set, not writes into a stored
        // plain object, so mutating in place would leave the polls panel showing a live vote.
        this.polls.set(key, { ...poll, outcome });
      }
      return;
    }

    // The defendant's private line to their lawyer. Rides its own viewport, so only those two
    // receive it; the "Prosecution" category groups it with the trial it belongs to.
    if ("MapLawyerChannel" in cmd) {
      const { channel_id, prosecution_id } = cmd.MapLawyerChannel;
      const key = slotKeyToString(channel_id);
      this.#channel_to_prosecution.set(key, slotKeyToString(prosecution_id));
      this.#map_channel(
        recipient,
        key,
        new_channel("Standard", "Prosecution", `lawyer-${prosecution_id.idx}v${prosecution_id.version}`),
      );
      return;
    }

    // Tag the trial channel: "Prosecution" category for rendering, plus a channel->prosecution
    // mapping for acting on it from inside. The snapshot itself is per-view; see #apply_to_view.
    //
    // The mapping is never untagged on close. It records that this channel WAS a trial's, which
    // ending the trial does not make untrue, and a view replaying the history later still needs it.
    if ("UpdateProsecution" in cmd) {
      const { prosecution_id, trial_channel } = cmd.UpdateProsecution;
      if (!trial_channel) return;

      const channel_key = slotKeyToString(trial_channel);
      this.#channel_to_prosecution.set(channel_key, slotKeyToString(prosecution_id));
      if (!this.channels.has(channel_key)) {
        this.#map_channel(recipient, channel_key, new_channel("Standard", "Prosecution", `trial-${prosecution_id.idx}v${prosecution_id.version}`));
      }
      return;
    }

    // Track a kidnapping. Kept after its reveal rather than deleted — the reveal carries only the
    // kidnapping's id, so resolving the victim from it is a lookup here, and a view replaying that
    // reveal later has to resolve the same victim. `revealed` is also what a live countdown would
    // stop on.
    if ("Kidnapping" in cmd) {
      const { kidnapping_id, target_id, duration } = cmd.Kidnapping;
      this.kidnappings.set(slotKeyToString(kidnapping_id), {
        victim: slotKeyToString(target_id),
        duration,
        revealed: false,
      });
      return;
    }

    if ("Incarceration" in cmd) {
      const { incarceration_id, victim_id, duration } = cmd.Incarceration;
      this.incarcerations.set(slotKeyToString(incarceration_id), {
        victim: slotKeyToString(victim_id),
        duration,
        released: false,
      });
      return;
    }

    if ("IncarcerationReleased" in cmd) {
      const key = slotKeyToString(cmd.IncarcerationReleased.incarceration_id);
      const incarceration = this.incarcerations.get(key);
      if (incarceration) {
        this.incarcerations.set(key, { ...incarceration, released: true });
      }
      return;
    }

    if ("KidnapReveal" in cmd) {
      const key = slotKeyToString(cmd.KidnapReveal.kidnapping_id);
      const kidnapping = this.kidnappings.get(key);
      // re-set rather than mutate: SvelteMap tracks its own get/set, not writes into a stored value.
      if (kidnapping) this.kidnappings.set(key, { ...kidnapping, revealed: true });
      return;
    }

    // The command that introduces a bug, addressed to the bug's own viewport — so it is also what
    // tells us which bug that viewport belongs to. The target is deliberately not carried; identity
    // leaks only through relayed message displays, so the feed is named by the bug's slot.
    //
    // Opening a viewer's gate is the per-view half, and it needs no hookup from this side: a view
    // holding the viewport now receives this live, and one entering later replays it out of the
    // viewport's history.
    if ("NewBug" in cmd) {
      const key = bugChannelKey(cmd.NewBug.bug_key);
      if (!this.bugs.has(key)) {
        this.bugs.set(key, new_channel("Bug", "Bug", `bug-${cmd.NewBug.bug_key.idx}v${cmd.NewBug.bug_key.version}`));
      }
      const viewport = recipientToViewport(recipient);
      if (viewport !== undefined) this.#viewport_to_bug.set(viewport, key);
      return;
    }

    // A relayed message captured by a bug. Stored globally; per-viewer visibility is the
    // visible_bugs gate. Rendered like any channel message — the sender display is the
    // target's own, which is what reveals them.
    if ("AddBugMessage" in cmd) {
      const { bug_key, display, content } = cmd.AddBugMessage;
      const bug = this.bugs.get(bugChannelKey(bug_key));
      bug?.events.push({ timestamp, data: { Message: { sender_display: display, content } } });
      return;
    }

    // The bug is no longer active: its feed goes read-only-archived but stays visible.
    if ("ArchiveBug" in cmd) {
      const bug = this.bugs.get(bugChannelKey(cmd.ArchiveBug.bug_key));
      if (bug) bug.archived = true;
      return;
    }

    // Personal-info commands go to two explicit recipients. This is the System copy, which feeds
    // the admin per-player inspector; the player's own copy lands in their Notifications log
    // (see #apply_to_view).
    if (recipient === "System") {
      if ("RoleUpdate" in cmd) {
        const key = slotKeyToString(cmd.RoleUpdate.target_id);
        this.player_info.set(key, { ...this.player_info.get(key), role: cmd.RoleUpdate.role });
        return;
      }
      if ("TrueNameUpdate" in cmd) {
        const key = slotKeyToString(cmd.TrueNameUpdate.target_id);
        this.player_info.set(key, { ...this.player_info.get(key), true_name: cmd.TrueNameUpdate.true_name });
        return;
      }
    }
  }

  // What a command does to ONE view's own state.
  //
  // Called once per receiving view, and REPLAYED for a view that gains the command's viewport
  // afterwards — so nothing here may destroy state it or a later replay depends on, and nothing
  // global belongs here (it would run once per view). See #apply and #backfill.
  #apply_to_view(view: GameView, { recipient, cmd, timestamp }: CommandPayload, pos: number) {
    // Record how far this view has been given the viewport, so a later entry by another of this
    // client's actors knows where its own gap begins.
    const viewport = recipientToViewport(recipient);
    if (viewport !== undefined) view.deliver_to(viewport, pos + 1);

    // Access gained. Everything previously addressed to the viewport is handed over right here,
    // out of the log — the server only sends a backfill for a viewport the whole CONNECTION lacked.
    if ("EnterViewport" in cmd) {
      const key = slotKeyToString(cmd.EnterViewport.viewport);
      view.viewports.add(key);
      this.#backfill(view, key, pos);
      return;
    }

    // Access lost. Nothing already received is dropped — this only means no more is coming.
    if ("ExitViewport" in cmd) {
      view.viewports.delete(slotKeyToString(cmd.ExitViewport.viewport));
      return;
    }

    // Tells this player whether they now own the gc. Drives the group-chat controls
    // (only the owner may add/remove/transfer).
    if ("GcOwnerStatus" in cmd) {
      const gc_key = slotKeyToString(cmd.GcOwnerStatus.gc_id);
      if (cmd.GcOwnerStatus.owner) view.owned_gcs.add(gc_key);
      else view.owned_gcs.delete(gc_key);
      return;
    }

    if ("UpdateChannelView" in cmd) {
      const channel_id = slotKeyToString(cmd.UpdateChannelView.channel_id);
      const p = cmd.UpdateChannelView.perms;
      const loggability_control = (p & PERM_LOGGABILITY) !== 0;
      const read = (p & PERM_VIEW) !== 0;
      const send = (p & PERM_SEND) !== 0;
      const existing = view.channel_views.get(channel_id);
      const old_perms = existing?.perms;
      let read_updated: number = timestamp;
      let had_positive = read || send || loggability_control;
      if (old_perms) {
        if (read === old_perms.read) {
          read_updated = old_perms.read_updated;
        }
        had_positive ||= old_perms.had_positive;
      }
      // Perms is the membership signal: an UpdateChannelView creates the channel
      // entry if absent, preserving members if it already existed. Re-set the map key
      // (rather than mutating in place) so perms/displays updates trigger reactivity.
      view.channel_views.set(channel_id, {
        perms: { had_positive, read_updated, loggability_control, read, send },
        members: existing?.members ?? new SvelteMap(),
        displays: cmd.UpdateChannelView.displays,
      });

      // Notify the viewer when they RECEIVE a notebook (any source): a notebook channel
      // going from no-read to read means the book is now in their hands. Frontend-derived,
      // no engine command. Fires once per gain (not on refreshes while already held).
      if (read && !(old_perms?.read ?? false) && this.#channel_to_notebook.has(channel_id)) {
        this.push_notif(view, recipient, timestamp, { NotebookReceived: {} });
      }
      return;
    }

    // The roster rides the channel's viewport rather than being re-sent per member, so it lands in
    // every view of ours that can read the channel. A view with no channel entry yet is skipped:
    // the entry is created by UpdateChannelView, which the engine emits first.
    if ("ShowChannelMember" in cmd) {
      const { channel_id, display, channel_perms } = cmd.ShowChannelMember;
      const entry = view.channel_views.get(slotKeyToString(channel_id));
      if (!entry) return;
      const key = displayKey(display);
      const had_positive =
        (entry.members.get(key)?.had_positive ?? false) || hasPositivePerms(channel_perms);
      entry.members.set(key, { display, perms: channel_perms, had_positive });
      return;
    }

    if ("RemoveChannelMember" in cmd) {
      const { channel_id, display } = cmd.RemoveChannelMember;
      view.channel_views.get(slotKeyToString(channel_id))?.members.delete(displayKey(display));
      return;
    }

    // A player's own abilities. An org's are held globally and handled in #apply_global; they
    // arrive on the org channel's viewport, so that is what tells the two apart.
    if ("UpdateAbilityView" in cmd) {
      if (viewport === undefined) upsert_ability(view.abilities, cmd.UpdateAbilityView);
      return;
    }

    if ("RemoveAbility" in cmd) {
      if (viewport === undefined) view.abilities.delete(slotKeyToString(cmd.RemoveAbility.ability_id));
      return;
    }

    // A passive the viewer now holds. Player-addressed in practice: an org's passives ride the org
    // channel's viewport, and orgs have no passive list to show them in yet, so those land in
    // whichever views can read that channel. Harmless, and it goes away when orgs get one.
    if ("UpdatePassiveView" in cmd) {
      const { passive_id, passive_type } = cmd.UpdatePassiveView;
      view.passives.set(slotKeyToString(passive_id), { type: passive_type });
      return;
    }

    if ("RemovePassive" in cmd) {
      view.passives.delete(slotKeyToString(cmd.RemovePassive.passive_id));
      return;
    }

    // This viewer's personal view of a poll they can see (eligibility + their own vote).
    if ("UpdatePollView" in cmd) {
      const { poll_id, eligible, own_vote } = cmd.UpdatePollView;
      view.poll_views.set(slotKeyToString(poll_id), { eligible, own_vote });
      return;
    }

    // The viewer's bug gate. Opened here rather than on EnterViewport, so the two routes into a
    // bug — holding the viewport when it is created, and entering the viewport afterwards — are
    // the same one line: the replay hands a late entrant this very command.
    if ("NewBug" in cmd) {
      view.visible_bugs.add(bugChannelKey(cmd.NewBug.bug_key));
      return;
    }

    // This viewer's prosecution snapshot, plus a news event when the phase differs from what this
    // view last held (a start when it's a new prosecution, an advance otherwise). Per-view rather
    // than global so each view diffs the stream IT receives — which is what makes an absent
    // player's backfill reproduce the ordered news timeline when they return.
    if ("UpdateProsecution" in cmd) {
      const { prosecution_id, prosecutor_display, defendant_display, lawyer_display, phase, trial_channel } = cmd.UpdateProsecution;
      const key = slotKeyToString(prosecution_id);
      const prev = view.prosecutions.get(key);
      view.prosecutions.set(key, {
        prosecutor_display,
        defendant_display,
        lawyer_display,
        phase,
        trial_channel: trial_channel ? slotKeyToString(trial_channel) : null,
        viewport: viewport ?? null,
      });
      if (!prev || !phaseViewEqual(prev.phase, phase)) {
        view.events.push({
          timestamp,
          data: { ProsecutionEvent: { prosecution_id: key, prosecutor_display, defendant_display, phase, ended: false } },
        });
      }
      return;
    }

    // The prosecution ended. If this view knew it, drop a terminal news event using the displays it
    // last held, and forget it. A view that was absent for the whole thing receives the ordered
    // timeline on entry and reaches the same place.
    if ("CloseProsecution" in cmd) {
      const key = slotKeyToString(cmd.CloseProsecution.prosecution_id);
      const prev = view.prosecutions.get(key);
      if (!prev) return;
      view.events.push({
        timestamp,
        data: { ProsecutionEvent: { prosecution_id: key, prosecutor_display: prev.prosecutor_display, defendant_display: prev.defendant_display, phase: prev.phase, ended: true } },
      });
      view.prosecutions.delete(key);
      return;
    }

    // ---- world events: this view's news feed ----

    if ("Death" in cmd) {
      const death = cmd.Death;
      view.events.push({
        timestamp,
        data: {
          Death: {
            target_id: slotKeyToString(death.target_id),
            true_name: death.true_name,
            death_message: death.death_message,
            role: death.role,
            notebook_transferred: death.notebook_transferred,
            ability_transferred: death.ability_transferred,
          }
        }
      });
      return;
    }

    if ("AnonymousAnnouncement" in cmd) {
      view.events.push({
        timestamp,
        data: { AnonymousAnnouncement: { content: cmd.AnonymousAnnouncement.content } },
      });
      return;
    }

    if ("Kidnapping" in cmd) {
      const { kidnapping_id, target_id, duration } = cmd.Kidnapping;
      view.events.push({
        timestamp,
        data: {
          Kidnapping: {
            kidnapping_id: slotKeyToString(kidnapping_id),
            target_id: slotKeyToString(target_id),
            duration,
          }
        },
      });
      return;
    }

    if ("Incarceration" in cmd) {
      const { incarceration_id, victim_id, duration } = cmd.Incarceration;
      view.events.push({
        timestamp,
        data: {
          Incarceration: {
            incarceration_id: slotKeyToString(incarceration_id),
            victim_id: slotKeyToString(victim_id),
            duration,
          }
        },
      });
      return;
    }

    if ("IncarcerationReleased" in cmd) {
      const incarceration_id = slotKeyToString(cmd.IncarcerationReleased.incarceration_id);
      view.events.push({
        timestamp,
        data: {
          IncarcerationReleased: {
            incarceration_id,
            victim: this.incarcerations.get(incarceration_id)?.victim ?? null,
          }
        },
      });
      return;
    }

    if ("KidnapReveal" in cmd) {
      const kidnapping_id = slotKeyToString(cmd.KidnapReveal.kidnapping_id);
      // The command names only the kidnapping, so the victim is resolved from the tracked one —
      // which is still there, because a reveal marks it rather than deleting it.
      view.events.push({
        timestamp,
        data: {
          KidnapReveal: {
            kidnapping_id,
            victim: this.kidnappings.get(kidnapping_id)?.victim ?? null,
            kidnapper: cmd.KidnapReveal.kidnapper ? slotKeyToString(cmd.KidnapReveal.kidnapper) : null,
          }
        },
      });
      return;
    }

    if ("PseudocideRevival" in cmd) {
      view.events.push({
        timestamp,
        data: { PseudocideRevival: { target_id: slotKeyToString(cmd.PseudocideRevival.target_id) } },
      });
      return;
    }

    // ---- directed personal events: this view's Notifications feed ----

    if ("RevealTrueName" in cmd) {
      // TODO(orgs): an org recipient should render this in the org's shared info channel, gated by
      // the same view perms as the org's channel, rather than in a single player's view.
      this.push_notif(view, recipient, timestamp, {
        RevealTrueName: {
          target_id: slotKeyToString(cmd.RevealTrueName.target_id),
          true_name: cmd.RevealTrueName.true_name,
        },
      });
      return;
    }

    if ("RevealNotebookHolding" in cmd) {
      this.push_notif(view, recipient, timestamp, {
        RevealNotebookHolding: {
          target_id: slotKeyToString(cmd.RevealNotebookHolding.target_id),
          holding: cmd.RevealNotebookHolding.holding,
        },
      });
      return;
    }

    // The viewer was told they've been bugged (directed to the target). A personal event, so it
    // lands in their Notifications channel — never News. Context only (never who).
    if ("Bugged" in cmd) {
      this.push_notif(view, recipient, timestamp, { Bugged: { context: cmd.Bugged.context } });
      return;
    }

    // The player's own copy of a personal-info command. The System copy feeds the admin inspector
    // and is handled in #apply_global.
    if (recipient !== "System") {
      if ("RoleUpdate" in cmd) {
        this.push_notif(view, recipient, timestamp, { RoleUpdate: { role: cmd.RoleUpdate.role } });
        return;
      }
      if ("TrueNameUpdate" in cmd) {
        this.push_notif(view, recipient, timestamp, { TrueNameUpdate: { true_name: cmd.TrueNameUpdate.true_name } });
        return;
      }
    }
  }

  // Apply a profile update: what the SERVER knows about who occupies a slot, on its own channel
  // beside the command stream.
  //
  // The server only ever sends these for actors whose MapPlayer this connection already received,
  // so an entry here is never the first we hear of a player. The guard below is not a permission
  // check — it is that ordering stated locally, so a profile arriving for an unknown slot is
  // dropped rather than conjuring a player out of the presentation channel.
  apply_profiles(update: ProfileUpdate) {
    for (const [id, profile] of update.profiles) {
      const key = slotKeyToString(id);
      const player = this.players.get(key);
      if (!player) continue;
      player.display_name = profile.display_name;
    }
  }

  system_view(): GameView {
    return this.views.get("System")!;
  }

  // Resolve a channel key to its Channel. "info:*" keys are frontend-only, read-only
  // info channels private to the viewer's own GameView; everything else is an
  // engine-backed channel from the shared top-level map.
  resolve_channel(viewer: string, key: string): Channel | undefined {
    if (key.startsWith("info:")) {
      const view = viewer === "Admin" ? this.system_view() : this.views.get(viewer);
      return view?.info_channels.get(key);
    }
    // Bug feeds are global (visibility is gated per-viewer at the list level, see Channels).
    if (key.startsWith("bug:")) {
      return this.bugs.get(key);
    }
    return this.channels.get(key);
  }

  // One view's single read-only Notifications info channel — the one home for every directed
  // personal event (reveals, bug alerts). Created lazily on first use.
  private notif_channel(view: GameView): Channel {
    let channel = view.info_channels.get(NOTIF_CHANNEL);
    if (!channel) {
      channel = new_channel("Info", "Personal", "Notifications");
      view.info_channels.set(NOTIF_CHANNEL, channel);
    }
    return channel;
  }

  // System's read-only mirror of one player's notifications, keyed per player so admin can
  // see every player's notification log side by side. Lives in the System view's info
  // channels under "info:notifs:<playerId>", named "notifications-<playername>".
  private system_player_notif_channel(player_key: string): Channel {
    const key = `${NOTIF_CHANNEL}:${player_key}`;
    const view = this.system_view();
    let channel = view.info_channels.get(key);
    if (!channel) {
      // Denormalised at creation, so a rename later leaves this channel under the old name. Fine
      // for now — it is admin-only scaffolding — but it is the same apply-time-resolution hazard
      // the poll opener had, and it wants a render-time lookup when this view is built properly.
      channel = new_channel("Info", "Personal", `Notifications-${playerLabel(player_key, this.players)}`);
      view.info_channels.set(key, channel);
    }
    return channel;
  }

  // Route a directed personal notification event into the target view's own Notifications
  // channel AND System's per-player mirror of it (so admin sees everyone's).
  private push_notif(
    view: GameView,
    recipient: CommandRecipient,
    timestamp: number,
    data: InfoEvent,
  ) {
    this.notif_channel(view).events.push({ timestamp, data });
    const player_key = recipientToPlayer(recipient);
    // only real players get a System mirror (skip org actors, which have no player entry)
    if (player_key && this.players.has(player_key)) {
      this.system_player_notif_channel(player_key).events.push({ timestamp, data });
    }
  }

  // The org ability list a command targets, or undefined if it is not an org's. An org's abilities
  // are addressed to its channel's viewport rather than to the org actor — "everyone in the org" is
  // expressed as "everyone who can see the org's channel".
  #org_abilities(recipient: CommandRecipient): SvelteMap<string, AbilityView> | undefined {
    const viewport = recipientToViewport(recipient);
    if (viewport === undefined) return undefined;
    const org_key = this.#viewport_to_org.get(viewport);
    return org_key ? this.orgs.get(org_key)?.abilities : undefined;
  }

  // Resolve an actor key to a display name — a player, or an org (a vote opener may be either,
  // e.g. an org-driven civilian arrest). Unknown keys fall back to "Unknown".
  actor_name(key: string): string {
    if (this.players.has(key)) return playerLabel(key, this.players);
    const org = this.orgs.get(key);
    if (org) return orgDisplayName(org.name);
    return "Unknown";
  }

  // Push a poll notice — "started" when outcome is null, else the resolution — into the channel the
  // poll's scope maps to: a channel directly, the world/news feed for AllPresent, or the org's
  // channel. No-op if that channel is unknown.
  private poll_notice(
    scope: PollVisibility,
    poll_id: string,
    subject: PollSubject,
    outcome: PollOutcome | null,
    timestamp: number,
    opener: string | null,
  ) {
    let channel_key: string | undefined;
    if (scope === "AllPresent") {
      channel_key = this.news_channel_id ?? undefined;
    } else if ("Channel" in scope) {
      channel_key = slotKeyToString(scope.Channel);
    } else {
      // Org-scoped: route to the org's backing channel.
      channel_key = this.orgs.get(slotKeyToString(scope.Org))?.channel_id;
    }
    if (!channel_key) return;
    const channel = this.channels.get(channel_key);
    if (!channel) return;
    channel.events.push({
      timestamp,
      data: { PollNotice: { poll_id, subject, outcome, opener } },
    });
  }

  find_abilities(viewer_key: string, name: string): string[] {
    const result: string[] = [];
    for (const [id, av] of this.views.get(viewer_key)?.abilities ?? []) {
      if (av.name === name) result.push(id);
    }
    return result;
  }

  // The notebook backing a notebook channel, if any. Used by the write menu to
  // target the correct notebook.
  notebook_for_channel(channel_key: string): NotebookKey | undefined {
    const notebook_key = this.#channel_to_notebook.get(channel_key);
    return notebook_key ? slotKeyFromString(notebook_key) : undefined;
  }

  // The group chat backing a channel, if any. Used by the group-chat controls to
  // target the correct gc. Returns the string key (use slotKeyFromString for actions).
  gc_key_for_channel(channel_key: string): string | undefined {
    return this.#channel_to_gc.get(channel_key);
  }

  // Resolve an ActorDisplay to the name to show. Raw displays look up the player's
  // name; the rest are self-describing or intentionally opaque.
  resolve_display(display: ActorDisplay): string {
    if (display === "Mysterious") return "???";
    if (display === "System") return "System";
    if ("Raw" in display)
      return playerLabel(slotKeyToString(display.Raw), this.players);
    if ("Role" in display) return display.Role;
    // Org display: resolve to the org's name (display.Org is its actor key).
    const org = this.orgs.get(slotKeyToString(display.Org));
    return org ? orgDisplayName(org.name) : "Org";
  }

  // The org backing a channel, if it is an org channel. Returns the string org key.
  org_key_for_channel(channel_key: string): string | undefined {
    return this.#channel_to_org.get(channel_key);
  }

  // Whether a channel is currently loggable (messages here can be autopsied / relayed to
  // bugs). A global channel property; defaults to false until a SetChannelLoggable arrives.
  is_channel_loggable(channel_key: string): boolean {
    return this.#channel_loggable.get(channel_key) ?? false;
  }

  // Whether a notebook is currently on loan (from NotebookBorrowingStatus). Defaults to false.
  is_notebook_borrowed(notebook_key: string): boolean {
    return this.#notebook_borrowed.get(notebook_key) ?? false;
  }

  // The viewport a channel's content rides, if we have seen it registered. Ask a view whether it
  // still holds this to know if the channel has gone quiet — see GameView.frozen.
  channel_viewport(channel_key: string): string | undefined {
    return this.#channel_to_viewport.get(channel_key);
  }

  // The prosecution a channel is the trial channel of, if any. For acting on the prosecution
  // from within its channel (rendering is driven by the "Prosecution" channel category instead).
  prosecution_key_for_channel(channel_key: string): string | undefined {
    return this.#channel_to_prosecution.get(channel_key);
  }
}

export const GAME_STATE_KEY = Symbol("game_state");
