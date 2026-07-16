import { SvelteMap, SvelteSet } from "svelte/reactivity";
import type { AbilityName, ActionRequest, ActorDisplay, ActorKey, ActionResponse, BugContext, BugKey, CommandPayload, CommandRecipient, NotebookKey, OrganizationName, PassiveType, PollOutcome, PollSubject, PollVisibility, ProsecutionKey, ProsecutionPhaseView, Role } from "./bindings";
import { slotKeyFromString, slotKeyToString } from "./bindings";
import { Sequencer } from "./lib/protocol";
import type { StreamingRouter, CommandBatch } from "./lib/protocol";

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
    target_id: string,
    duration: number,
  }
} | {
  KidnapReveal: {
    kidnapper: string | null, // null = the kidnapper stayed anonymous
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
  phase: ProsecutionPhaseView,
  trial_channel: string | null,
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
  // prosecution ids this viewer is currently frozen on (from FreezeProsecutionView): they lost
  // presence, so their snapshot above is stale until a live update replays. Any UpdateProsecution
  // they receive clears the id. Purely a "viewing frozen state" UI flag.
  frozen_prosecutions = new SvelteSet<string>();
  // bug channel keys ("bug:*") this viewer may currently see, from SetBugVisibility. The bug
  // logs themselves are global (GameState.bugs); this is the per-viewer visibility gate. Cleared
  // for everyone by ClearBugVisibily and dropped from all views by DeleteBug.
  visible_bugs = new SvelteSet<string>();

  // structuredClone can't clone a GameView: it drops the class prototype and
  // turns the SvelteMaps into plain Maps (losing reactivity), and can throw on
  // the maps' internal reactive state. Clone by hand into fresh SvelteMaps,
  // copying each value object so views don't share mutable state.
  clone(): GameView {
    const copy = new GameView();
    for (const [id, cv] of this.channel_views) {
      const members = new SvelteMap<string, ChannelMemberView>();
      for (const [dkey, m] of cv.members) {
        members.set(dkey, {
          display: m.display,
          perms: m.perms,
          had_positive: m.had_positive,
        });
      }
      copy.channel_views.set(id, {
        perms: { ...cv.perms },
        members,
        displays: [...cv.displays],
      });
    }
    for (const [id, ability] of this.abilities) {
      copy.abilities.set(id, $state.snapshot(ability));
    }
    for (const [id, pv] of this.passives) {
      copy.passives.set(id, { ...pv });
    }
    for (const gc of this.owned_gcs) {
      copy.owned_gcs.add(gc);
    }
    for (const [id, pv] of this.poll_views) {
      copy.poll_views.set(id, { ...pv });
    }
    for (const [id, pd] of this.prosecutions) {
      copy.prosecutions.set(id, { ...pd });
    }
    for (const id of this.frozen_prosecutions) {
      copy.frozen_prosecutions.add(id);
    }
    for (const key of this.visible_bugs) {
      copy.visible_bugs.add(key);
    }
    for (const event of this.events) {
      copy.events.push(event);
    }
    for (const [key, ch] of this.info_channels) {
      const copy_ch = new_channel(ch.kind, ch.category, ch.name);
      copy_ch.archived = ch.archived;
      for (const event of ch.events) copy_ch.events.push(event);
      copy.info_channels.set(key, copy_ch);
    }
    return copy;
  }
}

// Whether two prosecution phase-views are the same. Subphases collapse into the view already
// (Grace/Presentation both read as e.g. Trial:Prosecutor), so this is what "the phase changed".
export function phaseViewEqual(a: ProsecutionPhaseView, b: ProsecutionPhaseView): boolean {
  if (typeof a === "string" || typeof b === "string") return a === b;
  return a.Trial === b.Trial;
}

// Stable map key for an ActorDisplay (the tagged union isn't usable as a key directly).
export function displayKey(d: ActorDisplay): string {
  if (typeof d === "string") return d; // "Mysterious" | "System"
  if ("Raw" in d) return `Raw:${slotKeyToString(d.Raw)}`;
  if ("Org" in d) return `Org:${slotKeyToString(d.Org)}`;
  return `Role:${d.Role}`;
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

export interface Player {
  display_name: string;
}

// Admin-facing per-player facts, populated from the System copy of the personal-info commands
// (RoleUpdate / TrueNameUpdate). In the real server only the host receives the System copy, so
// this stays empty on ordinary player clients. Surfaced when admin inspects a player.
export interface PlayerInfo {
  role?: Role;
  true_name?: string;
}

export function new_player(display_name: string): Player {
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

// Maps a command recipient to the key of the view it targets. Actor recipients
// key by their slot; the string variants map to the standing views: System is the
// admin view (world events emitted with include_system land here), BasePlayer is
// the Base template. Returns undefined only for an unhandled recipient.
function recipientToView(rec: CommandRecipient): string | undefined {
  if (typeof rec !== "string") {
    return slotKeyToString(rec.Actor);
  }
  if (rec === "System") return "System";
  if (rec === "BasePlayer") return "Base";
  return undefined;
}

function recipientToPlayer(recipient: CommandRecipient): string | undefined {
  if (typeof (recipient) !== 'string') {
    const id = recipient.Actor;
    return slotKeyToString(id);
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
  // visible_bugs. The relayed AddBugMessage stream is System-directed, hence global here.
  bugs = new SvelteMap<string, Channel>();

  // Transport is injected (attach). The router is the swap seam; the client model
  // doesn't care whether it's Tauri or a websocket to the server.
  #router: StreamingRouter | null = null;
  // All state changes — replies to our own requests AND pushed external-action
  // batches — funnel through one ordered pipe so they can never race into a desync.
  #seq = new Sequencer();

  constructor() {
    this.views.set("Base", new GameView());
    // System (admin) is not a player: it's never cloned from Base and bypasses
    // channel perms (see is_admin), so its perms/abilities stay empty. It exists
    // to hold the state authority can't cover, like the world-event stream.
    this.views.set("System", new GameView());
  }

  // Wire up the transport. Pushed external-action command batches are fed into the
  // same seq-ordered pipe as our own replies. Returns nothing; call once at startup.
  attach(router: StreamingRouter) {
    this.#router = router;
    router.onCommands((batch: CommandBatch) =>
      this.#seq.ingest({
        seq: batch.seq,
        run: () => this.apply_batch(batch.commands),
      }),
    );
  }

  // Fire this client's own action. Returns an error string on failure (for UX), or
  // void on success. The reply's state effect (response data + commands) is applied
  // through the same seq-ordered pipe as external batches — never inline — so a reply
  // that arrives ahead of a still-pending external batch waits its turn instead of
  // desyncing. The error, being UX only, is read and returned immediately.
  async dispatch(
    request: ActionRequest,
    args?: Record<string, unknown>,
  ): Promise<string | void> {
    if (!this.#router) throw new Error("dispatch before attach");
    const { seq, execution } = await this.#router.sendAction(request);
    const { exec_result } = execution;

    if (exec_result === "Crashed") {
      // The transport still consumed a seq for this action, but a crash carries no
      // commands to apply. Feed the Sequencer a no-op so `#last` advances past it —
      // otherwise this seq is a permanent gap and every later batch (e.g. the next
      // AddPlayer reply) stays buffered behind it, silently, even though the engine
      // rebooted and is responding.
      this.#seq.ingest({ seq, run: () => {} });
      return "The engine has crashed.";
    }
    const result = exec_result.Standard;
    if ("Err" in result) {
      // Even on failure the engine returns catchup commands (job-queue world
      // progression) that must still be applied. Only then surface the error.
      const [error, context] = result.Err;
      this.#seq.ingest({ seq, run: () => this.apply_batch(context.commands) });
      return String(error);
    }
    const [response, context] = result.Ok;
    this.#seq.ingest({
      seq,
      run: () => {
        this.handle_response(response, args);
        this.apply_batch(context.commands);
      },
    });
  }

  // Apply a batch of commands in push order. The public seam the Sequencer drives;
  // command ordering within a batch is significant (create-before-reference,
  // last-write-wins perms), so never reorder.
  apply_batch(commands: CommandPayload[]) {
    for (const cmd of commands) {
      this.handle_command(cmd);
    }
  }

  private new_view(key: string) {
    this.views.set(key, this.base_view().clone());
  }

  private handle_command({ recipient, cmd, timestamp }: CommandPayload) {
    if ("MapLounge" in cmd) {
      const { channel_id, contact_id } = cmd.MapLounge;
      // Lounges are identified by their contact-channel id.
      const name = `lounge-${contact_id}`;

      this.channels.set(slotKeyToString(channel_id), new_channel("Standard", "Lounge", name));

      return;
    }

    if ("MapGc" in cmd) {
      const { gc_id, channel_id, contact_id } = cmd.MapGc;
      const channel_key = slotKeyToString(channel_id);
      // Same shape as lounges for now (custom gc names arrive with the server).
      const name = `groupchat-${contact_id}`;

      this.channels.set(channel_key, new_channel("Standard", "Groupchat", name));
      this.#channel_to_gc.set(channel_key, slotKeyToString(gc_id));

      return;
    }

    // Per-recipient: tells this player whether they now own the gc. Drives the
    // group-chat controls (only the owner may add/remove/transfer).
    if ("GcOwnerStatus" in cmd) {
      const player_id = recipientToPlayer(recipient);
      if (player_id) {
        const view = this.views.get(player_id);
        const gc_key = slotKeyToString(cmd.GcOwnerStatus.gc_id);
        if (cmd.GcOwnerStatus.owner) view?.owned_gcs.add(gc_key);
        else view?.owned_gcs.delete(gc_key);
      }
      return;
    }

    // Register an org and its backing channel (an "Org"-kind channel, mirroring MapGc).
    if ("MapOrg" in cmd) {
      const { org_id, channel_id, org_name } = cmd.MapOrg;
      const channel_key = slotKeyToString(channel_id);
      const org_key = slotKeyToString(org_id);

      this.channels.set(channel_key, new_channel("Standard", "Org", orgDisplayName(org_name)));
      this.#channel_to_org.set(channel_key, org_key);
      this.orgs.set(org_key, new_org(org_name, channel_key));
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

      this.channels.set(
        slotKeyToString(channel_id),
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

      this.channels.set(key, new_channel("Standard", category, channel_name));

      return;
    }

    // Register a personal channel: a real, sendable channel private to its owner. Global like
    // the other channel maps; per-viewer visibility falls out of channel-view perms (only the
    // owner is a member). Kind "Personal" so the sidebar groups it with the info feeds.
    if ("MapPersonalChannel" in cmd) {
      const { channel_id } = cmd.MapPersonalChannel;
      const key = slotKeyToString(channel_id);
      if (!this.channels.has(key)) {
        this.channels.set(key, new_channel("Standard", "Personal", `personal-${channel_id.idx}v${channel_id.version}`));
      }
      return;
    }

    // this should only ever be targetted toward players. ignore anything else.
    if ("UpdateChannelView" in cmd) {
      const channel_id = slotKeyToString(cmd.UpdateChannelView.channel_id);

      const player_id = recipientToPlayer(recipient);
      if (player_id) {
        const view = this.views.get(player_id)!;
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
        const perms: ChannelPerms = {
          had_positive,
          read_updated,
          loggability_control,
          read,
          send,
        };
        // Perms is the membership signal: an UpdateChannelView creates the channel
        // entry if absent, preserving members if it already existed. Re-set the map key
        // (rather than mutating in place) so perms/displays updates trigger reactivity.
        view.channel_views.set(channel_id, {
          perms,
          members: existing?.members ?? new SvelteMap(),
          displays: cmd.UpdateChannelView.displays,
        });

        // Notify the viewer when they RECEIVE a notebook (any source): a notebook channel
        // going from no-read to read means the book is now in their hands. Frontend-derived,
        // no engine command. Fires once per gain (not on refreshes while already held).
        if (read && !(old_perms?.read ?? false) && this.#channel_to_notebook.has(channel_id)) {
          this.push_notif(recipient, timestamp, { NotebookReceived: {} });
        }
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

    // RemoveChannel is per-player: the target player is no longer a member (e.g. a
    // notebook that transferred away), so drop the channel from THAT player's view
    // only. The channel still exists globally for whoever else holds it — deleting it
    // globally here would wipe it for the new owner who was just granted access.
    if ("RemoveChannel" in cmd) {
      const player_id = recipientToPlayer(recipient);
      if (player_id) {
        // Drops both perms and members for that player — they're no longer a member.
        this.views
          .get(player_id)
          ?.channel_views.delete(slotKeyToString(cmd.RemoveChannel.channel_id));
      }
      return;
    }

    // DeleteChannel removes the channel globally (system-directed); the channel
    // ceases to exist for everyone.
    if ("DeleteChannel" in cmd) {
      const key = slotKeyToString(cmd.DeleteChannel.channel_id);
      this.channels.delete(key);
      this.#channel_loggable.delete(key);
      return;
    }

    // A member (identified by a display) was made visible in a channel. Only members
    // of the channel receive these, so the channel entry already exists (perms first).
    if ("ShowChannelMember" in cmd) {
      const { channel_id, display, channel_perms } = cmd.ShowChannelMember;
      const view = this.views.get(recipientToView(recipient) ?? "");
      const entry = view?.channel_views.get(slotKeyToString(channel_id));
      if (entry) {
        const key = displayKey(display);
        const had_positive =
          (entry.members.get(key)?.had_positive ?? false) ||
          hasPositivePerms(channel_perms);
        entry.members.set(key, { display, perms: channel_perms, had_positive });
      }
      return;
    }

    if ("RemoveChannelMember" in cmd) {
      const { channel_id, display } = cmd.RemoveChannelMember;
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.channel_views
        .get(slotKeyToString(channel_id))
        ?.members.delete(displayKey(display));
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

    if ("UpdateAbilityView" in cmd) {
      const { ability_id, ability_name, success_usages_remaining, failure_usages_remaining, iterations_to_reset } = cmd.UpdateAbilityView;
      // An org-owned ability is directed to Actor(org); route it to the org's shared
      // ability list rather than a player view.
      const abilities = this.abilities_for_recipient(recipient);
      if (abilities) {
        const id = slotKeyToString(ability_id);
        const existing = abilities.get(id);
        if (existing) {
          existing.success_usages_remaining = success_usages_remaining;
          existing.failure_usages_remaining = failure_usages_remaining;
          existing.iterations_to_reset = iterations_to_reset;
        } else {
          let av: AbilityView = $state({ name: ability_name, success_usages_remaining, failure_usages_remaining, iterations_to_reset });
          abilities.set(id, av);
        }
      }
      return;
    }

    if ("RemoveAbility" in cmd) {
      this.abilities_for_recipient(recipient)?.delete(slotKeyToString(cmd.RemoveAbility.ability_id));
      return;
    }

    // A passive the recipient now holds. Directed to the owner's player view (orgs don't
    // display passives yet, so an org recipient resolves to no view and is dropped).
    if ("UpdatePassiveView" in cmd) {
      const { passive_id, passive_type } = cmd.UpdatePassiveView;
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.passives.set(slotKeyToString(passive_id), { type: passive_type });
      return;
    }

    if ("RemovePassive" in cmd) {
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.passives.delete(slotKeyToString(cmd.RemovePassive.passive_id));
      return;
    }

    // Shared poll data (global). First sighting drops a "started" notice into the poll's
    // scoped channel; later UpdatePolls just refresh the tally.
    if ("UpdatePoll" in cmd) {
      const { poll_id, subject, scope, accept, reject, potential } = cmd.UpdatePoll;
      const key = slotKeyToString(poll_id);
      if (!this.polls.has(key)) {
        this.poll_notice(scope, key, subject, null, timestamp);
      }
      this.polls.set(key, { subject, scope, accept, reject, potential });
      return;
    }

    // A poll concluded: drop the shared data and every viewer's personal view, and drop a
    // resolution notice into its scoped channel.
    if ("ClosePoll" in cmd) {
      const { poll_id, outcome } = cmd.ClosePoll;
      const key = slotKeyToString(poll_id);
      const poll = this.polls.get(key);
      if (poll) this.poll_notice(poll.scope, key, poll.subject, outcome, timestamp);
      this.polls.delete(key);
      for (const view of this.views.values()) view.poll_views.delete(key);
      return;
    }

    // This player's personal view of a poll they can see (eligibility + their own vote).
    if ("UpdatePollView" in cmd) {
      const { poll_id, eligible, own_vote } = cmd.UpdatePollView;
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.poll_views.set(slotKeyToString(poll_id), { eligible, own_vote });
      return;
    }

    // This player lost view of the poll's scope: drop their personal view so the poll
    // disappears from their Polls panel. The shared data stays for other viewers.
    if ("RemovePollView" in cmd) {
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.poll_views.delete(slotKeyToString(cmd.RemovePollView.poll_id));
      return;
    }

    // This viewer's prosecution snapshot. Globally, tag the trial channel ("Prosecution" kind for
    // rendering + a channel->prosecution mapping for acting on it). Per-view: store the snapshot,
    // clear the frozen notice, and emit a news event when the phase differs from what this view
    // last held (a start when it's a new prosecution, an advance otherwise). Per-view diffing is
    // what makes an absent player's deferred replay reproduce the ordered news timeline.
    if ("UpdateProsecution" in cmd) {
      const { prosecution_id, prosecutor_display, defendant_display, phase, trial_channel } = cmd.UpdateProsecution;
      const key = slotKeyToString(prosecution_id);
      const channelKey = trial_channel ? slotKeyToString(trial_channel) : null;
      if (channelKey) {
        this.#channel_to_prosecution.set(channelKey, key);
        if (!this.channels.has(channelKey)) {
          this.channels.set(channelKey, new_channel("Standard", "Prosecution", `trial-${prosecution_id.idx}v${prosecution_id.version}`));
        }
      }
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.frozen_prosecutions.delete(key);
        const prev = view.prosecutions.get(key);
        view.prosecutions.set(key, { prosecutor_display, defendant_display, phase, trial_channel: channelKey });
        if (!prev || !phaseViewEqual(prev.phase, phase)) {
          view.events.push({
            timestamp,
            data: { ProsecutionEvent: { prosecution_id: key, prosecutor_display, defendant_display, phase, ended: false } },
          });
        }
      }
      return;
    }

    // The prosecution ended. Per-view (so absent players get it deferred, in order): if this view
    // knew the prosecution, drop a terminal news event using its last-held displays, forget it,
    // and untag its channel. Also clear the frozen flag.
    if ("CloseProsecution" in cmd) {
      const key = slotKeyToString(cmd.CloseProsecution.prosecution_id);
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        const prev = view.prosecutions.get(key);
        if (prev) {
          view.events.push({
            timestamp,
            data: { ProsecutionEvent: { prosecution_id: key, prosecutor_display: prev.prosecutor_display, defendant_display: prev.defendant_display, phase: prev.phase, ended: true } },
          });
          if (prev.trial_channel) this.#channel_to_prosecution.delete(prev.trial_channel);
          view.prosecutions.delete(key);
        }
        view.frozen_prosecutions.delete(key);
      }
      return;
    }

    // This viewer lost presence: they're viewing frozen prosecution state until a live update
    // replays. Purely a UI notice; the snapshot itself is untouched.
    if ("FreezeProsecutionView" in cmd) {
      const view = this.views.get(recipientToView(recipient) ?? "");
      view?.frozen_prosecutions.add(slotKeyToString(cmd.FreezeProsecutionView.prosecution_id));
      return;
    }

    if ("Death" in cmd) {
      const death = cmd.Death;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
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
      }
    }

    if ("AnonymousAnnouncement" in cmd) {
      const annc = cmd.AnonymousAnnouncement;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            AnonymousAnnouncement: {
              content: annc.content
            }
          }
        });
      }
    }

    if ("Kidnapping" in cmd) {
      const kidnapping = cmd.Kidnapping;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            Kidnapping: {
              target_id: slotKeyToString(kidnapping.target_id),
              duration: kidnapping.duration,
            }
          }
        });
      }
    }

    if ("KidnapReveal" in cmd) {
      const reveal = cmd.KidnapReveal;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            KidnapReveal: {
              kidnapper: reveal.kidnapper
                ? slotKeyToString(reveal.kidnapper)
                : null,
            }
          }
        });
      }
    }

    if ("PseudocideRevival" in cmd) {
      const revival = cmd.PseudocideRevival;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        view.events.push({
          timestamp,
          data: {
            PseudocideRevival: {
              target_id: slotKeyToString(revival.target_id),
            }
          }
        });
      }
    }

    if ("RevealTrueName" in cmd) {
      const reveal = cmd.RevealTrueName;
      // TODO(orgs): when the recipient actor is an organization, this should render in
      // the org's shared info channel, gated by the same view perms as the org's lounge
      // channel (so each member sees it iff they can see that channel) — not in a single
      // player's view. Orgs aren't modeled on the frontend yet, so route to the player
      // view for now (TrueNameInvite pushes this to the org actor, which has no view).
      this.push_notif(recipient, timestamp, {
        RevealTrueName: {
          target_id: slotKeyToString(reveal.target_id),
          true_name: reveal.true_name,
        },
      });
    }

    if ("RevealNotebookHolding" in cmd) {
      const reveal = cmd.RevealNotebookHolding;
      this.push_notif(recipient, timestamp, {
        RevealNotebookHolding: {
          target_id: slotKeyToString(reveal.target_id),
          holding: reveal.holding,
        },
      });
    }

    // A bug was created (System-directed). Register its global surveillance feed. The
    // target is deliberately not carried — identity leaks only through relayed message
    // displays — so the feed is named by the bug's slot.
    if ("NewBug" in cmd) {
      const key = bugChannelKey(cmd.NewBug.bug_key);
      if (!this.bugs.has(key)) {
        this.bugs.set(key, new_channel("Bug", "Bug", `bug-${cmd.NewBug.bug_key.idx}v${cmd.NewBug.bug_key.version}`));
      }
      return;
    }

    // A relayed message captured by a bug (System-directed → stored globally). Rendered like
    // any channel message; the sender display is the target's own, which is what reveals them.
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

    // Hide a bug from everyone at once (visibility revoked globally). The feed itself stays;
    // only the per-viewer visibility is cleared.
    if ("ClearBugVisibily" in cmd) {
      const key = bugChannelKey(cmd.ClearBugVisibily.bug_id);
      for (const view of this.views.values()) view.visible_bugs.delete(key);
      return;
    }

    // The bug should never have existed: drop the feed and every viewer's visibility of it.
    if ("DeleteBug" in cmd) {
      const key = bugChannelKey(cmd.DeleteBug.bug_id);
      this.bugs.delete(key);
      for (const view of this.views.values()) view.visible_bugs.delete(key);
      return;
    }

    // Per-viewer bug visibility (directed to the owner / custody viewer). Grants or revokes
    // this viewer's access to the bug's feed.
    if ("SetBugVisibility" in cmd) {
      const { bug_id, visible } = cmd.SetBugVisibility;
      const view = this.views.get(recipientToView(recipient) ?? "");
      if (view) {
        const key = bugChannelKey(bug_id);
        if (visible) view.visible_bugs.add(key);
        else view.visible_bugs.delete(key);
      }
      return;
    }

    // The viewer was told they've been bugged (directed to the target). A personal event, so
    // it lands in their Notifications info channel — never News. Context only (never who).
    if ("Bugged" in cmd) {
      this.push_notif(recipient, timestamp, { Bugged: { context: cmd.Bugged.context } });
      return;
    }

    // Personal-info commands go to two explicit recipients: the player copy (Actor) lands in
    // their Notifications log (and System's per-player mirror); the System copy feeds the
    // admin per-player inspector (player_info).
    if ("RoleUpdate" in cmd) {
      const { target_id, role } = cmd.RoleUpdate;
      if (recipient === "System") {
        const key = slotKeyToString(target_id);
        this.player_info.set(key, { ...this.player_info.get(key), role });
      } else {
        this.push_notif(recipient, timestamp, { RoleUpdate: { role } });
      }
      return;
    }

    if ("TrueNameUpdate" in cmd) {
      const { target_id, true_name } = cmd.TrueNameUpdate;
      if (recipient === "System") {
        const key = slotKeyToString(target_id);
        this.player_info.set(key, { ...this.player_info.get(key), true_name });
      } else {
        this.push_notif(recipient, timestamp, { TrueNameUpdate: { true_name } });
      }
      return;
    }
  }

  private handle_response(response: ActionResponse, args?: Record<string, unknown>) {
    if ("AddPlayer" in response) {
      this.add_player(response.AddPlayer.id, args?.display_name as string);
      return;
    }
  }

  base_view(): GameView {
    return this.views.get("Base")!;
  }

  system_view(): GameView {
    return this.views.get("System")!;
  }

  add_player(id: ActorKey, display_name: string) {
    const key = slotKeyToString(id);
    this.new_view(key);
    this.players.set(key, new_player(display_name));
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

  // The viewer's single read-only Notifications info channel — the one home for every
  // directed personal event (reveals, bug alerts). Created lazily on first use. Returns
  // undefined when the recipient maps to no known view (e.g. an org actor, not modeled yet).
  private notif_channel(view_key: string | undefined): Channel | undefined {
    const view = view_key ? this.views.get(view_key) : undefined;
    if (!view) return undefined;
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
      const name = this.players.get(player_key)?.display_name ?? player_key;
      channel = new_channel("Info", "Personal", `Notifications-${name}`);
      view.info_channels.set(key, channel);
    }
    return channel;
  }

  // Route a directed personal notification event into the recipient player's own
  // Notifications channel AND System's per-player mirror of it (so admin sees everyone's).
  private push_notif(
    recipient: CommandRecipient,
    timestamp: number,
    data: InfoEvent,
  ) {
    this.notif_channel(recipientToView(recipient))?.events.push({ timestamp, data });
    const player_key = recipientToPlayer(recipient);
    // only real players get a System mirror (skip org actors, which have no player entry)
    if (player_key && this.players.has(player_key)) {
      this.system_player_notif_channel(player_key).events.push({ timestamp, data });
    }
  }

  // Push a poll notice — "started" when outcome is null, else the resolution — into the
  // channel the poll's scope maps to: a channel directly, the world/news feed for
  // AllPresent, or (once orgs exist) the org's channel. No-op if that channel is unknown.
  private poll_notice(
    scope: PollVisibility,
    poll_id: string,
    subject: PollSubject,
    outcome: PollOutcome | null,
    timestamp: number,
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
      data: { PollNotice: { poll_id, subject, outcome } },
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
      return this.players.get(slotKeyToString(display.Raw))?.display_name ?? "Unknown";
    if ("Role" in display) return display.Role;
    // Org display: resolve to the org's name (display.Org is its actor key).
    const org = this.orgs.get(slotKeyToString(display.Org));
    return org ? orgDisplayName(org.name) : "Org";
  }

  // The abilities map a directed ability command targets: an org's shared list when the
  // recipient is a known org, otherwise the player view's own list.
  private abilities_for_recipient(
    recipient: CommandRecipient,
  ): SvelteMap<string, AbilityView> | undefined {
    const key = recipientToView(recipient) ?? "";
    return this.orgs.get(key)?.abilities ?? this.views.get(key)?.abilities;
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

  // The prosecution a channel is the trial channel of, if any. For acting on the prosecution
  // from within its channel (rendering is driven by the "Prosecution" channel category instead).
  prosecution_key_for_channel(channel_key: string): string | undefined {
    return this.#channel_to_prosecution.get(channel_key);
  }
}

export const GAME_STATE_KEY = Symbol("game_state");
