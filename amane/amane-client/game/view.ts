// ONE actor's world, and nothing else's. System is a view like any other: it holds what was
// addressed to it (the Admin recipient) and nothing more.
//
// This is the whole correctness argument of the client. A view holds only what it was delivered,
// so it cannot render something it was never told — not because a render site remembered to check,
// but because the state isn't there to read. Nothing here is shared with another view; the router
// in game.ts decides who receives a command and each recipient applies it into its own copy.
//
// The cost is duplication: two actors in the same channel hold two copies of its messages. That is
// the price of the guarantee, and it is bounded by what was actually delivered rather than by the
// size of the game.
//
// Plain data. Handlers write it, queries and surfaces read it, and nothing here notifies anyone:
// the session bumps the client's version once per batch.
import type { PrivilegeSet, ProsecutionSide, Role, Statuses } from "../bindings.ts";
import type {
  AbilityView,
  ActionLogEntry,
  Channel,
  ChannelView,
  GameEvent,
  InfoEvent,
  Notebook,
  Org,
  PassiveView,
  Player,
  PollData,
  PollView,
  ProsecutionData,
  TrackedIncarceration,
  TrackedKidnapping,
} from "./types.ts";

// The single per-view Notifications feed: where every event directed at this actor lands.
export const NOTIF_CHANNEL = "info:notifs";

export class View {
  // This view's own actor key, or "System" for the admin view. What a mention tests itself
  // against, and how the view knows which orgs it belongs to.
  readonly own_key: string;
  // Learned from this view's own RoleUpdate. Null until told.
  own_role: Role | null = null;

  constructor(own_key: string) {
    this.own_key = own_key;
  }

  // ---- objects this view has been told about ----
  channels = new Map<string, Channel>();
  players = new Map<string, Player>();
  orgs = new Map<string, Org>();
  notebooks = new Map<string, Notebook>();
  polls = new Map<string, PollData>();
  prosecutions = new Map<string, ProsecutionData>();
  kidnappings = new Map<string, TrackedKidnapping>();
  incarcerations = new Map<string, TrackedIncarceration>();

  // Facts that reach a view without the object they are about, so they stay keyed maps.
  //
  // A channel as this view stands in it: a member without read access is told their names but
  // never the channel.
  channel_views = new Map<string, ChannelView>();
  // StatusFlag bitmask per actor: public condition, carried on the world-data viewport.
  actor_statuses = new Map<string, Statuses>();
  // Poll participation, directed to this view.
  poll_views = new Map<string, PollView>();
  // Which prosecutions this view is a party to, and on which side. The public snapshot cannot say
  // (an anonymous prosecutor is Mysterious in their own copy of it), and either may land first.
  own_prosecutions = new Map<string, ProsecutionSide>();

  // ---- the world ----
  // World events: this view's news feed. Death beats may be stamped in the future; the feed query
  // hides them until the game clock gets there.
  events: GameEvent[] = [];
  news_anchor: string | null = null;
  press_conf = new Set<string>();
  // The News channel's key. Kept rather than looked up: world events render into News whether or
  // not the channel itself is still held.
  news_channel: string | null = null;
  // Game time as of a real wall-clock `sent_at`. See game_time_now.
  game_clock: { time: number; sent_at: number } | null = null;

  // ---- this actor's own standing ----
  abilities = new Map<string, AbilityView>();
  passives = new Map<string, PassiveView>();
  // StateFlag bitmask, replaced wholesale.
  states = 0;
  owned_gcs = new Set<string>();
  // Personal: the rest of the org is never told who is an OG.
  og_orgs = new Set<string>();

  // ---- System only ----
  // The whole key ledger, replaced wholesale on every delivery.
  keys = new Map<string, PrivilegeSet>();
  action_log: ActionLogEntry[] = [];

  // ---- viewports ----

  // Held RIGHT NOW. Routes a Viewport-addressed command here, and answers the live half of frozen.
  viewports = new Set<string>();
  // Held EVER. Never shrinks: losing a viewport stops the delivery, it does not unsee what was
  // already delivered.
  seen_viewports = new Set<string>();
  // What world events ride. Left when this view loses presence or the world goes dark; both read
  // the same way, as "this is the last thing you were told".
  world_events_viewport: string | null = null;

  // viewport -> the log position, exclusive, up to which THIS view has been given that viewport's
  // commands. Per-view on purpose: the server's equivalent is per-connection, so a connection
  // holding several actors is sent a viewport's history once, and this is what lets the second
  // actor be handed its own copy.
  #watermark = new Map<string, number>();

  delivered(viewport: string): number {
    return this.#watermark.get(viewport) ?? 0;
  }

  deliver_to(viewport: string, position: number) {
    this.#watermark.set(viewport, Math.max(this.delivered(viewport), position));
  }

  // Has state that arrived through this viewport stopped moving? Nothing is retracted: what was
  // received stays, shown as the last thing heard rather than passed off as current. Always false
  // for System, which never enters a viewport and so never leaves one.
  frozen(viewport: string | null): boolean {
    return viewport !== null && this.seen_viewports.has(viewport) && !this.viewports.has(viewport);
  }

  // Game time now: the anchor's game time plus the real time elapsed since it was sent. Zero
  // before an anchor arrives.
  game_time_now(): number {
    if (!this.game_clock) return 0;
    return this.game_clock.time + (Date.now() - this.game_clock.sent_at);
  }

  // The org whose channel rides this viewport, if any. "Everyone in the org" is expressed as
  // "everyone who can see the org's channel", so an org-addressed command finds its org this way.
  org_at(viewport: string | undefined): Org | undefined {
    if (viewport === undefined) return undefined;
    for (const org of this.orgs.values()) {
      if (org.viewport === viewport) return org;
    }
    return undefined;
  }

  // Every directed personal event lands here. Created on first use.
  push_notif(timestamp: number, data: InfoEvent) {
    let channel = this.channels.get(NOTIF_CHANNEL);
    if (!channel) {
      channel = {
        kind: "Info",
        category: "Personal",
        name: "Notifications",
        archived: false,
        loggable: false,
        viewport: null,
        link: null,
        events: [],
      };
      this.channels.set(NOTIF_CHANNEL, channel);
    }
    channel.events.push({ timestamp, data });
  }
}
