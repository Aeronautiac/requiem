// ONE actor's world, and nothing else's.
//
// This is the whole correctness argument of the client. A view holds only what it was delivered,
// so it cannot render something it was never told — not because a render site remembered to check,
// but because the state isn't there to read. Nothing here is shared with another view; the router
// in state.svelte.ts decides who receives a command and each recipient applies it into its own
// copy.
//
// The cost is duplication: two actors in the same channel hold two copies of its messages. That is
// the price of the guarantee, and it is bounded by what was actually delivered rather than by the
// size of the game.
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import type {
  ActionOutcome,
  ActionRequest,
  ActorDisplay,
  ActorKey,
  LogCommand,
  LogType,
  PrivilegeSet,
  Profile,
  ProsecutionSide,
  Role,
  Statuses,
} from "../bindings";
import { slotKeyToString } from "../bindings";
import { NOTIF_CHANNEL, logDumpKey, logDumpLabel, new_channel, orgDisplayName, playerLabel, roleLabel, t } from "./helpers.svelte";
import { commandToEvent } from "./commands/events";
import type {
  AbilityView,
  Channel,
  ChannelView,
  GameEvent,
  InfoEvent,
  Org,
  PassiveView,
  Player,
  PlayerInfo,
  PollData,
  PollView,
  ProsecutionData,
  TrackedIncarceration,
  TrackedKidnapping,
} from "./types";

// Exactly one entry on the host's action timeline: an action request a connection submitted and
// how it came out, as the server recorded them at the time it was asked. The request is "who did
// what, as whom"; the outcome is how the engine (or the gate in front of it) answered.
export interface ActionLogEntry {
  // Monotonic per view, assigned at append time. The timeline keys its rows off this rather than
  // content: entries are appended in delivery order and never reordered, but their `time:variant`
  // is NOT unique — a manual game-clock that hasn't ticked can stamp many identical actions with
  // the same instant, which would collide as duplicate `{#each}` keys. `id` is the one field
  // guaranteed distinct.
  id: number;
  time: number;
  action: ActionRequest;
}

export class GameView {
  // This view's own actor key ("System" for the admin view). What a mention tests itself against,
  // and how the view knows which orgs it belongs to (membership is org.members.has(own_key)).
  readonly own_key: string;
  // This view's own role, learned from its personal RoleUpdate. Null until told. A `@<role:…>`
  // mention pings this view iff it names this role.
  own_role = $state<Role | null>(null);

  constructor(own_key: string) {
    this.own_key = own_key;
  }

  // ---- objects this view has been told about ----
  channels = new SvelteMap<string, Channel>();
  players = new SvelteMap<string, Player>();
  orgs = new SvelteMap<string, Org>();
  polls = new SvelteMap<string, PollData>();
  bugs = new SvelteMap<string, Channel>();
  contact_logs = new SvelteMap<string, Channel>();
  kidnappings = new SvelteMap<string, TrackedKidnapping>();
  incarcerations = new SvelteMap<string, TrackedIncarceration>();
  // Current news anchor (null = vacant) and the press-conference roster — the people who may speak
  // on the news. Both are live state kept current from the world-events feed, alongside the feed
  // announcement of each change.
  news_anchor = $state<string | null>(null);
  press_conf = new SvelteSet<string>();
  // Only ever populated on System, which is the one view the personal-info System copies reach.
  player_info = new SvelteMap<string, PlayerInfo>();
  // StatusFlag bitmask per actor: the public condition of every player this view can see, carried
  // on the world-data viewport and re-emitted on every change. Keyed by slot rather than stored on
  // the Player because a world sweep drives it and the two arrive by different routes.
  actor_statuses = new SvelteMap<string, Statuses>();

  // News must appear to exist even after the underlying channel is removed, so the key is kept
  // rather than read off `channels`. $state so a component resolving it recomputes the moment it
  // is assigned; otherwise selecting news before the channel exists never picks up its perms.
  news_channel_id = $state<string | null>(null);

  // ---- this actor's own standing ----
  channel_views = new SvelteMap<string, ChannelView>();
  events: GameEvent[] = $state([]); // world events: this view's news feed

  // Push a run of world events keyed to their own timestamps: one whose time has already passed
  // lands at once, a future one after the clock reaches it. Used to stagger a single command (a
  // death) into timed reveals. The feed sorts by timestamp, so each part still lands in order.
  //
  // The timestamps are GAME time, and the reveals are re-evaluated against the live game clock by a
  // reaper rather than a one-shot setTimeout sized from the clock at staging time. That matters
  // because a forward time skip (the server winding the clock ahead) hands a view its deaths from
  // within the skip BEFORE the new clock anchor that follows them: if the delay were computed
  // against that stale pre-skip clock, a death would schedule itself hours out and never surface
  // even though the skip has already left it behind. Re-checked against the clock -- and flushed the
  // moment a new anchor lands -- a death whose time the skip has passed arrives immediately. A
  // replayed command is "all there" already -- its parts are all in the past -- so it plays out at
  // once, with no notion of "live" needed.
  stage_world_events(parts: GameEvent[]) {
    for (const part of parts) {
      if (part.timestamp <= this.game_time_now()) this.events.push(part);
      else this.#pending_events.push(part);
    }
    this.#ingest_pending();
  }

  // Events staged for a moment the game clock has not reached yet. Held until it does; this is what
  // lets a delayed reveal survive the clock jumping under it (see stage_world_events).
  #pending_events: GameEvent[] = [];
  #reaper: ReturnType<typeof setInterval> | undefined;

  // Start (or keep) the reaper while anything is pending, and opportunistically reap right away --
  // so a clock anchor that just jumped the game forward drains the queue immediately rather than
  // waiting for the next interval tick.
  #ingest_pending() {
    if (this.#pending_events.length > 0 && this.#reaper === undefined) {
      this.#reaper = setInterval(() => this.#reap_pending(), 250);
    }
    this.#reap_pending();
  }

  // Reveal every pending event whose time has arrived. Order among the revealed ones is preserved
  // (the feed re-sorts by timestamp anyway), and the reaper stops once the queue empties.
  #reap_pending() {
    const now = this.game_time_now();
    const still_pending: GameEvent[] = [];
    for (const part of this.#pending_events) {
      if (part.timestamp <= now) this.events.push(part);
      else still_pending.push(part);
    }
    this.#pending_events = still_pending;
    if (this.#pending_events.length === 0 && this.#reaper !== undefined) {
      clearInterval(this.#reaper);
      this.#reaper = undefined;
    }
  }

  // The current game time, from the clock anchor the server sent: game time at `sent_at`, plus one
  // millisecond of game time per real millisecond elapsed since. Before an anchor has arrived the
  // game is taken to be at 0 -- its start.
  game_time_now(): number {
    const c = this.game_clock;
    if (!c) return 0;
    return c.time + (Date.now() - c.sent_at);
  }
  // Read-only feeds this view builds for itself rather than being delivered, keyed "info:*".
  info_channels = new SvelteMap<string, Channel>();
  abilities = new SvelteMap<string, AbilityView>();
  passives = new SvelteMap<string, PassiveView>();
  // StateFlag bitmask. Replaced wholesale: the command carries the entire set. What this view
  // knows about ANOTHER actor's state comes from the event that caused it, never from here.
  states = $state(0);
  owned_gcs = new SvelteSet<string>();
  // Personal: the rest of the org is never told who is an OG, so this answers for yourself only.
  og_orgs = new SvelteSet<string>();
  poll_views = new SvelteMap<string, PollView>();
  prosecutions = new SvelteMap<string, ProsecutionData>();
  // Personal: which prosecutions this view is a party to, and on which side. The public snapshot
  // cannot say — an anonymous prosecutor is Mysterious in their own copy of it — so the engine
  // tells each side directly. Kept separately from `prosecutions` because it is a different fact
  // arriving on a different route, and either may land first.
  own_prosecutions = new SvelteMap<string, ProsecutionSide>();

  // ---- indices ----
  //
  // The engine addresses a thing by one key and the UI needs to get back to another. All of these
  // are learned from the command that introduced the object, so a view holds an entry only for
  // objects it was told about.
  #channel_notebook = new SvelteMap<string, string>();
  #notebook_channel = new SvelteMap<string, string>();
  #channel_gc = new SvelteMap<string, string>();
  #channel_lounge = new SvelteMap<string, string>();
  #channel_org = new SvelteMap<string, string>();
  #org_channel = new SvelteMap<string, string>();
  #channel_prosecution = new SvelteMap<string, string>();
  // Kept apart from the Channel object so they land whether or not the establishing Map* has
  // arrived yet.
  #channel_loggable = new SvelteMap<string, boolean>();
  #notebook_borrowed = new SvelteMap<string, boolean>();
  // Only the original owner (and admin) is ever told this, so absence is meaningful: a borrower or
  // inheritor holds no entry and is left to deduce the book's nature.
  #notebook_fake = new SvelteMap<string, boolean>();
  // Set once the notebook's channel is archived, which is exactly how a notebook is destroyed: the
  // book's only physical home is its channel, so an archived notebook channel is a destroyed book.
  // Stored here rather than read off the channel so the fact reads identically anywhere the view is
  // asked, and so a destroyed book is not offered as something to write in or pass.
  #notebook_destroyed = new SvelteMap<string, boolean>();
  // viewport -> the org whose backing channel it is. Routing rather than visibility: an org's
  // abilities are addressed to that viewport rather than to the org actor, so this is how a
  // command finds the org it belongs to.
  #viewport_org = new SvelteMap<string, string>();

  map_notebook(channel_key: string, notebook_key: string) {
    this.#channel_notebook.set(channel_key, notebook_key);
    this.#notebook_channel.set(notebook_key, channel_key);
  }
  notebook_of(channel_key: string): string | undefined {
    return this.#channel_notebook.get(channel_key);
  }
  channel_of_notebook(notebook_key: string): string | undefined {
    return this.#notebook_channel.get(notebook_key);
  }
  is_notebook_channel(channel_key: string): boolean {
    return this.#channel_notebook.has(channel_key);
  }

  map_gc(channel_key: string, gc_key: string) {
    this.#channel_gc.set(channel_key, gc_key);
  }
  gc_of(channel_key: string): string | undefined {
    return this.#channel_gc.get(channel_key);
  }

  map_lounge(channel_key: string, lounge_key: string) {
    this.#channel_lounge.set(channel_key, lounge_key);
  }
  lounge_of(channel_key: string): string | undefined {
    return this.#channel_lounge.get(channel_key);
  }

  // Bidirectional, like the notebook pair: the org's own record holds engine facts only, so which
  // channel backs it lives here with every other channel link.
  map_org_channel(channel_key: string, org_key: string) {
    this.#channel_org.set(channel_key, org_key);
    this.#org_channel.set(org_key, channel_key);
  }
  org_of_channel(channel_key: string): string | undefined {
    return this.#channel_org.get(channel_key);
  }
  channel_of_org(org_key: string): string | undefined {
    return this.#org_channel.get(org_key);
  }

  // Never untagged on close. It records that this channel WAS a trial's, which ending the trial
  // does not make untrue, and a replay later still needs it.
  map_prosecution_channel(channel_key: string, prosecution_id: string) {
    this.#channel_prosecution.set(channel_key, prosecution_id);
  }
  prosecution_of_channel(channel_key: string): string | undefined {
    return this.#channel_prosecution.get(channel_key);
  }

  set_loggable(channel_key: string, loggable: boolean) {
    this.#channel_loggable.set(channel_key, loggable);
  }
  // Whether messages here can be autopsied or relayed to bugs. False until told otherwise.
  is_loggable(channel_key: string): boolean {
    return this.#channel_loggable.get(channel_key) ?? false;
  }

  set_notebook_borrowed(notebook_key: string, borrowed: boolean) {
    this.#notebook_borrowed.set(notebook_key, borrowed);
  }
  is_notebook_borrowed(notebook_key: string): boolean {
    return this.#notebook_borrowed.get(notebook_key) ?? false;
  }

  set_notebook_fake(notebook_key: string, fake: boolean) {
    this.#notebook_fake.set(notebook_key, fake);
  }
  // undefined = this view was never told, which is not the same as "genuine".
  notebook_fake(notebook_key: string): boolean | undefined {
    return this.#notebook_fake.get(notebook_key);
  }

  set_notebook_destroyed(notebook_key: string, destroyed: boolean) {
    this.#notebook_destroyed.set(notebook_key, destroyed);
  }
  is_notebook_destroyed(notebook_key: string): boolean {
    return this.#notebook_destroyed.get(notebook_key) ?? false;
  }

  record_org_viewport(viewport: string | undefined, org_key: string) {
    if (viewport !== undefined) this.#viewport_org.set(viewport, org_key);
  }
  // The org a viewport-addressed command belongs to, or undefined if it is not an org's. "Everyone
  // in the org" is expressed as "everyone who can see the org's channel".
  org_at(viewport: string | undefined): Org | undefined {
    if (viewport === undefined) return undefined;
    const key = this.#viewport_org.get(viewport);
    return key ? this.orgs.get(key) : undefined;
  }
  // The channel an org-addressed command lands in — for the answers an org gets as a body rather
  // than any one member, which have nowhere personal to go.
  org_channel_at(viewport: string | undefined): string | undefined {
    if (viewport === undefined) return undefined;
    const key = this.#viewport_org.get(viewport);
    return key ? this.channel_of_org(key) : undefined;
  }

  // ---- viewports ----

  // Held RIGHT NOW. Routes a Viewport-addressed command here, and answers the live half of frozen.
  viewports = new SvelteSet<string>();
  // Held EVER. Never shrinks: losing a viewport stops the delivery, it does not unsee what was
  // already delivered.
  //
  // These two are the only per-object visibility state there is. Whether a bug feed, a contact
  // log, a poll or a channel may be shown, and whether what it shows is still current, are both
  // answered from here via `viewport_of` below — so no object kind carries a presence set of its
  // own, and none can quietly forget to ask.
  seen_viewports = new SvelteSet<string>();

  // object -> the viewport its content rides, learned from the recipient of whichever command
  // introduced the object.
  //
  // Channels, bug feeds and contact logs share one key space (the one `channel` takes). Polls and
  // prosecutions are separate slot spaces whose ids would collide with channel keys, so they are
  // namespaced here; callers go through the accessors below and never see a prefix.
  #viewport_of = new SvelteMap<string, string>();

  // viewport key -> the log position, exclusive, up to which THIS view has been given that
  // viewport's commands. Per-VIEW, which is the whole point: the server's equivalent is
  // per-connection, so a connection holding several actors is sent a viewport's history exactly
  // once. This is what lets the second actor be handed its own copy.
  #watermark = new Map<string, number>();

  delivered(viewport: string): number {
    return this.#watermark.get(viewport) ?? 0;
  }

  deliver_to(viewport: string, position: number) {
    this.#watermark.set(viewport, Math.max(this.delivered(viewport), position));
  }

  // Note which viewport an object's content rides. No-op for a command that names no viewport,
  // which leaves the object unregistered and so never frozen — the safe reading, since nothing
  // then claims its state has stopped.
  record_viewport(viewport: string | undefined, key: string) {
    if (viewport !== undefined) this.#viewport_of.set(key, viewport);
  }

  // The viewport a channel, bug feed or contact log rides.
  viewport_of(channel_key: string): string | undefined {
    return this.#viewport_of.get(channel_key);
  }

  poll_viewport(poll_id: string): string | undefined {
    return this.#viewport_of.get(POLL_PREFIX + poll_id);
  }

  prosecution_viewport(prosecution_id: string): string | undefined {
    return this.#viewport_of.get(PROSECUTION_PREFIX + prosecution_id);
  }

  // What world events ride. A view that has left it still holds every event it was given but is
  // hearing nothing further, which is the one thing a news feed must not imply otherwise.
  //
  // Left for two different reasons that look identical from here: this view lost presence, or the
  // world went dark. Both mean the same thing to a reader — what you are looking at is the last
  // thing you were told — so neither needs telling apart to render it honestly.
  world_events_viewport(): string | undefined {
    return this.#viewport_of.get(WORLD_EVENTS);
  }

  record_poll_viewport(viewport: string | undefined, poll_id: string) {
    this.record_viewport(viewport, POLL_PREFIX + poll_id);
  }

  record_prosecution_viewport(viewport: string | undefined, prosecution_id: string) {
    this.record_viewport(viewport, PROSECUTION_PREFIX + prosecution_id);
  }

  record_world_events_viewport(viewport: string) {
    this.#viewport_of.set(WORLD_EVENTS, viewport);
  }

  // May this view see what rode this viewport at all? Held once is enough — what was delivered
  // stays delivered.
  visible(viewport: string | null | undefined): boolean {
    return viewport != null && this.seen_viewports.has(viewport);
  }

  // Has state that arrived through this viewport stopped moving?
  //
  // A viewport this view no longer holds will deliver nothing further, so everything it did
  // deliver is the last thing heard rather than the current state. Not specific to any one kind of
  // state — a channel, an org, a bug feed and a prosecution all go stale the same way.
  //
  // Nothing is retracted: what was received stays. This is the difference between showing someone
  // what they knew and lying to them about what is.
  frozen(viewport: string | null | undefined): boolean {
    if (viewport == null) return false;
    return this.seen_viewports.has(viewport) && !this.viewports.has(viewport);
  }

  // ---- lookups the components ask for ----

  // "info:*" keys are feeds this view built; everything else was delivered to it.
  channel(key: string): Channel | undefined {
    if (key.startsWith("info:")) return this.info_channels.get(key);
    if (key.startsWith("bug:")) return this.bugs.get(key);
    if (key.startsWith("contacts:")) return this.contact_logs.get(key);
    if (key.startsWith("autopsy:") || key.startsWith("tapin:")) return this.logs.get(key);
    return this.channels.get(key);
  }

  // The one home for every directed personal event. Created on first use.
  notif_channel(): Channel {
    let channel = this.info_channels.get(NOTIF_CHANNEL);
    if (!channel) {
      channel = new_channel("Info", "Personal", "Notifications");
      this.info_channels.set(NOTIF_CHANNEL, channel);
    }
    return channel;
  }

  push_notif(timestamp: number, data: InfoEvent) {
    this.notif_channel().events.push({ timestamp, data });
  }

  // ---- names ----

  // What the SERVER knows about who occupies the slots this view already holds, routed here by the
  // same gates as any command. A view cannot learn a name before it learns the slot — the server
  // only ever sends a profile for an actor whose MapActor this view already received — so the name
  // is written straight onto the held player, never stashed for a slot that may not exist.
  apply_profiles(profiles: [ActorKey, Profile][]) {
    for (const [id, profile] of profiles) {
      const player = this.players.get(slotKeyToString(id));
      if (player) player.display_name = profile.display_name;
    }
  }

  // ---- keys ----

  // The whole key ledger, as the server keeps it: every key and what its holder may do. Admin-gated,
  // so this fills the System view only. Replaced wholesale on every delivery — the roster is whole,
  // never a diff — so a stale entry over a revoked key is gone the moment the new set lands.
  keys = new SvelteMap<string, PrivilegeSet>();

  apply_keys(keys: [string, PrivilegeSet][]) {
    this.keys.clear();
    for (const [id, privileges] of keys) this.keys.set(id, privileges);
  }

  // ---- game clock ----

  // The game's clock anchor, riding the world-data viewport like the ProfileRoster: game time as of
  // a real wall `sent_at`. Every view that can read the world holds one, and the UI derives current
  // game time from `time + (Date.now() - sent_at)`.
  game_clock = $state<{ time: number; sent_at: number } | null>(null);

  set_game_clock(sent_at: number, time: number) {
    this.game_clock = { sent_at, time };
    // A new anchor can jump the game forward (a time skip); drain anything that staging scheduled
    // against a stale clock the moment the corrected time is known.
    this.#ingest_pending();
  }

  // ---- log records ----

  // The host's action timeline, appended in delivery order and never reordered. Admin-gated, so it
  // fills only the System view. Kept separate from the engine-command log because it answers a
  // different question -- "what was asked, and how it went" -- rather than "what the world now is".
  action_log: ActionLogEntry[] = $state([]);

  // Next action_timeline id. Kept off the entries themselves so a replayed timeline, which rebuilds
  // by appending, assigns the same ids as the live one did.
  #action_seq = 0;

  apply_log_action(action: ActionRequest, time: number) {
    this.action_log.push({ id: ++this.#action_seq, time, action });
  }

  // A filtered channel record this view has been handed: an autopsy of a target's record, or a
  // tapped channel's log. Rendered as a read-only feed exactly like a bug log — a Log channel whose
  // events are normalized record data — so it flows through the same sidebar + channel-view path.
  logs = new SvelteMap<string, Channel>();

  apply_log_dump(log_type: LogType, entries: LogCommand[]) {
    const key = logDumpKey(log_type);
    let feed = this.logs.get(key);
    if (!feed) {
      // Name by what the record is. `view.channel` routes autopsy:/tapin: keys here, so the
      // sidebar and the channel view both resolve it without any special-casing.
      feed = new_channel("Log", "Logs", logDumpLabel(key, this));
      this.logs.set(key, feed);
    }
    for (const lc of entries) {
      const event = commandToEvent(lc.data, lc.time);
      if (event) feed.events.push(event);
    }
  }

  // An actor key to the name this view knows it by. A vote opener may be a player or an org.
  actor_name(key: string): string {
    if (this.players.has(key)) return playerLabel(key, this.players);
    const org = this.orgs.get(key);
    return org ? orgDisplayName(org.name) : t("display_unknown");
  }

  // A display to the name to show. Raw looks up a player; the rest are self-describing or
  // intentionally opaque. A Role display reads by its display name, not the raw variant.
  resolve_display(display: ActorDisplay): string {
    if (display === "Mysterious") return t("display_mysterious");
    if (display === "System") return t("display_system");
    if ("Raw" in display) return playerLabel(slotKeyToString(display.Raw), this.players);
    if ("Role" in display) return roleLabel(display.Role);
    const org = this.orgs.get(slotKeyToString(display.Org));
    return org ? orgDisplayName(org.name) : t("display_org_unknown");
  }

  find_abilities(name: string): string[] {
    const out: string[] = [];
    for (const [id, ability] of this.abilities) {
      if (ability.name === name) out.push(id);
    }
    return out;
  }
}

const POLL_PREFIX = "poll:";
const PROSECUTION_PREFIX = "prosecution:";
// The world's own viewports belong to no object, so they are filed under fixed keys rather than an
// id. More join this when rulesets land.
//
// Only the events one is filed: it is the only viewport whose loss has to be shown. World data is
// ungated — every player holds it from creation and never leaves — so nothing about it is ever
// stale and there is nothing to answer.
const WORLD_EVENTS = "world:events";
