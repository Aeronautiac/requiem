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
import type { ActorDisplay, ProsecutionSide, Statuses } from "../bindings";
import { slotKeyToString } from "../bindings";
import { NOTIF_CHANNEL, new_channel, orgDisplayName, playerLabel, t } from "./helpers.svelte";
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

export class GameView {
  // ---- objects this view has been told about ----
  channels = new SvelteMap<string, Channel>();
  players = new SvelteMap<string, Player>();
  orgs = new SvelteMap<string, Org>();
  polls = new SvelteMap<string, PollData>();
  bugs = new SvelteMap<string, Channel>();
  contact_logs = new SvelteMap<string, Channel>();
  kidnappings = new SvelteMap<string, TrackedKidnapping>();
  incarcerations = new SvelteMap<string, TrackedIncarceration>();
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

  // An actor key to the name this view knows it by. A vote opener may be a player or an org.
  actor_name(key: string): string {
    if (this.players.has(key)) return playerLabel(key, this.players);
    const org = this.orgs.get(key);
    return org ? orgDisplayName(org.name) : t("display_unknown");
  }

  // A display to the name to show. Raw looks up a player; the rest are self-describing or
  // intentionally opaque.
  resolve_display(display: ActorDisplay): string {
    if (display === "Mysterious") return t("display_mysterious");
    if (display === "System") return t("display_system");
    if ("Raw" in display) return playerLabel(slotKeyToString(display.Raw), this.players);
    if ("Role" in display) return display.Role;
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
