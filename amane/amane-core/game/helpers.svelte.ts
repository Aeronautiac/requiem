// Pure functions shared by the command handlers and the components. Nothing here holds state or
// reaches for a view — everything it needs is an argument, which is what lets a component call it
// without the state layer and a handler call it without the DOM.
import type {
  AbilityKey,
  AbilityName,
  ActorDisplay,
  BugKey,
  ChannelProfileView,
  ContactLogType,
  LogType,
  OrganizationName,
  PassiveType,
  PollSubject,
  ProsecutionPhaseView,
  Role,
  ServerOutput,
  Statuses,
} from "../bindings";
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { slotKeyFromString, slotKeyToString, StatusFlag } from "../bindings";
import type { ExecError } from "../lib/protocol";
import type { GameView } from "./view.svelte";
import { STRINGS, type StringKey } from "../config/strings";
import type {
  AbilityView,
  Channel,
  ChannelCategory,
  ChannelKind,
  ChannelPerms,
  Org,
  Player,
} from "./types";

// How a refusal reads. The pipeline hands back a value; this is the single place it becomes words,
// so a call site never invents its own wording for the same failure.
//
// An unrecognised refusal code is shown raw rather than swallowed: an engine error nobody has
// written copy for is still more use on screen than a blank.
export function execErrorText(error: ExecError): string {
  switch (error.kind) {
    case "denied":
      return t("exec_denied");
    case "crashed":
      return t("exec_crashed");
    case "desync":
      return t("exec_desync");
    case "refused": {
      const key = `control_${error.code}` as StringKey;
      return key in STRINGS ? t(key) : error.code;
    }
  }
}

// Resolve a copy key, filling `{name}` placeholders. The only way a string reaches the screen.
export function t(key: StringKey, vars?: Record<string, string | number>): string {
  const template: string = STRINGS[key];
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in vars ? String(vars[name]) : whole,
  );
}

// ---- recipients (from an output's view gates) ----

// The server resolved an engine recipient into one or more gates and filtered them against the
// connection's privileges; these are what the client routes on. A gate list can name several
// players and/or viewports, so the helpers return arrays and the router iterates every gate
// rather than assuming a single recipient.

// The viewports this output is addressed to, in gate order. Empty for a command with none.
export function gateViewports(out: ServerOutput): string[] {
  const out_views: string[] = [];
  for (const gate of out.view_gates) {
    if (typeof gate !== "string" && "Viewport" in gate) {
      out_views.push(slotKeyToString(gate.Viewport));
    }
  }
  return out_views;
}

// The actors this output is addressed to, in gate order. Empty for a command addressed to none.
export function gateActors(out: ServerOutput): string[] {
  const out_actors: string[] = [];
  for (const gate of out.view_gates) {
    if (typeof gate !== "string" && "Player" in gate) {
      out_actors.push(slotKeyToString(gate.Player));
    }
  }
  return out_actors;
}

// Whether this output is addressed to an administrator (System view).
export function gateAdmin(out: ServerOutput): boolean {
  return out.view_gates.includes("Admin");
}

// The stable per-view key for a log record, derived from what it is. An autopsy is of one actor's
// record; a tap-in is of one channel by contact id.
export function logDumpKey(log_type: LogType): string {
  if ("Autopsy" in log_type) return `autopsy:${slotKeyToString(log_type.Autopsy)}`;
  return `tapin:${log_type.TapIn}`;
}

// Whether a sidebar/selection key names a log record rather than a real channel. Both feed and log
// surfaces route on these keys, so the prefix is the one thing they share.
export function isLogDumpKey(key: string): boolean {
  return key.startsWith("autopsy:") || key.startsWith("tapin:");
}

// The name a log record shows in a sidebar or header, resolved against the view it belongs to.
export function logDumpLabel(key: string, view: GameView): string {
  if (key.startsWith("autopsy:")) {
    return `Autopsy: ${view.actor_name(key.slice("autopsy:".length))}`;
  }
  if (key.startsWith("tapin:")) {
    return `Tap In: ${key.slice("tapin:".length)}`;
  }
  return "Record";
}

// ---- channel keys ----

// Bugs live in their own BugKey slot space, which can collide with real ChannelKeys, so the prefix
// keeps them separate in the channel-keyed resolve path.
export function bugChannelKey(bug_id: BugKey): string {
  return `bug:${slotKeyToString(bug_id)}`;
}

// There are exactly three contact-log records (Full/Even/Odd), keyed by which one they are rather
// than by any passive — the record is a world singleton, and the same feed reaches a linked reader
// who never sees which passive fed it.
export function contactLogChannelKey(kind: ContactLogType): string {
  return `contacts:${kind}`;
}

// The single per-viewer Notifications feed, and the one place every directed-at-you personal event
// lands. NOT News (world events), and NOT a "personal channel" (that's a real engine channel). Add
// new personal event kinds here rather than spawning a second one.
export const NOTIF_CHANNEL = "info:notifs";

// A feed you are handed rather than a room you are in. None are engine channels, so none carry
// perms, loggability or a send box — the one question each render site asks.
export function isReadOnlyKind(kind: ChannelKind): boolean {
  return kind === "Info" || kind === "Bug" || kind === "ContactLog" || kind === "Log";
}

// ---- constructors ----

// Must be a $state proxy, not a plain object: a Map tracks its own get/set, not deep mutations to
// stored values, so channel.events.push would not trigger reactivity.
export function new_channel(kind: ChannelKind, category: ChannelCategory, name: string): Channel {
  const channel: Channel = $state({ kind, category, name, archived: false, events: [] });
  return channel;
}

export function new_player(display_name: string | null): Player {
  const player: Player = $state({ display_name });
  return player;
}

export function new_org(name: OrganizationName): Org {
  // $state, like new_player: `leader` is a plain scalar, so without a reactive proxy around the
  // object a change to it would update the value but never re-render. The Set/Map members are
  // reactive on their own; the scalar is what needs this.
  const org: Org = $state({
    name,
    leader: null,
    members: new SvelteSet<string>(),
    effective: new SvelteSet<string>(),
    abilities: new SvelteMap<string, AbilityView>(),
  });
  return org;
}

// Create or refresh one entry in an ability list. Shared because the same UpdateAbilityView lands
// either in a player's own list or in an org's shared one, depending only on how it was addressed.
// Takes the command's fields directly: the engine's variant is an inline struct, so bindings has
// no name for it.
export function upsert_ability(
  abilities: Map<string, AbilityView>,
  { ability_id, ability_name, success_usages_remaining, failure_usages_remaining, iterations_to_reset, base_reset, unlimited }: {
    ability_id: AbilityKey;
    ability_name: AbilityName;
    success_usages_remaining: number;
    failure_usages_remaining: number;
    iterations_to_reset: number;
    base_reset: number;
    unlimited: boolean;
  },
) {
  const id = slotKeyToString(ability_id);
  const existing = abilities.get(id);
  if (existing) {
    existing.success_usages_remaining = success_usages_remaining;
    existing.failure_usages_remaining = failure_usages_remaining;
    existing.iterations_to_reset = iterations_to_reset;
    existing.base_reset = base_reset;
    existing.unlimited = unlimited;
    return;
  }
  const view: AbilityView = $state({
    name: ability_name,
    success_usages_remaining,
    failure_usages_remaining,
    iterations_to_reset,
    base_reset,
    unlimited,
  });
  abilities.set(id, view);
}

// ---- permissions ----

// Channel permission bits, mirroring ChannelPerm in the engine.
export const PERM_SEND = 1;
export const PERM_VIEW = 2;
export const PERM_LOGGABILITY = 4;

// What this view may do in a channel, folded over every name it holds there.
//
// Permissions belong to the PROFILE, not to the person: the same viewer may be able to talk under
// one of their names and not another. This is the question "can I do X here at all", which is what
// the sidebar and the composer's enabled state ask; sending itself asks the chosen name.
// What one name may do, for display beside it. Loggability control is deliberately absent: it is
// about the channel rather than about standing in it, and it belongs to the toggle it drives.
export function permsLabel(perms: number): string {
  const parts: string[] = [];
  if (perms & PERM_VIEW) parts.push("read");
  if (perms & PERM_SEND) parts.push("send");
  return parts.join(" · ");
}

// The active flags of an actor's public status, as short badge labels. `missing` is the blackout
// blur, and never appears next to the specific flags it stands in for, because the engine withholds
// those the moment it sets it.
export function statusLabels(status: Statuses): string[] {
  const labels: string[] = [];
  if (status & StatusFlag.Missing) labels.push("missing");
  if (status & StatusFlag.Dead) labels.push("dead");
  if (status & StatusFlag.Incarcerated) labels.push("incarcerated");
  if (status & StatusFlag.Kidnapped) labels.push("kidnapped");
  if (status & StatusFlag.Custody) labels.push("custody");
  if (status & StatusFlag.Ipp) labels.push("ipp");
  if (status & StatusFlag.Bugged) labels.push("bugged");
  return labels;
}

export function ownPerms(own: ChannelProfileView[]): ChannelPerms {
  const all = own.reduce((acc, profile) => acc | profile.perms, 0);
  return {
    read: (all & PERM_VIEW) !== 0,
    send: (all & PERM_SEND) !== 0,
    loggability_control: (all & PERM_LOGGABILITY) !== 0,
  };
}

// ---- naming ----

// Stable map key for an ActorDisplay (the tagged union isn't usable as a key directly).
export function displayKey(d: ActorDisplay): string {
  if (typeof d === "string") return d; // "Mysterious" | "System"
  if ("Raw" in d) return `Raw:${slotKeyToString(d.Raw)}`;
  if ("Org" in d) return `Org:${slotKeyToString(d.Org)}`;
  return `Role:${d.Role}`;
}

// A name as it should be READ: every word capitalised, nothing else touched.
//
// Names reach the client in whatever shape they were stored. True names are folded to lowercase by
// the engine so that guessing one is not a spelling contest, and a display name is whatever somebody
// typed into a box. Neither is a presentation decision, and both render as a name.
//
// PRESENTATION ONLY. Nothing derived from this may be sent back or compared against anything — the
// stored copy is the name, and this is a rendering of it.
export function nameLabel(name: string): string {
  return name.replace(
    /\S+/g,
    (word) => word[0].toUpperCase() + word.slice(1).toLowerCase(),
  );
}

// Falls back to a generated label rather than a bare key, matching how every other unnamed object
// in the UI reads ("lounge-3", "trial-1v0").
export function playerLabel(id: string, players: ReadonlyMap<string, Player>): string {
  const name = players.get(id)?.display_name;
  if (name) return nameLabel(name);
  const key = slotKeyFromString(id);
  return t("player_unnamed", { idx: key.idx, version: key.version });
}

// Falls back to the raw config code, so an org added to the engine before the copy exists still
// renders as something rather than blank.
export function orgDisplayName(name: OrganizationName): string {
  const key = `org_name_${name}` as StringKey;
  return key in STRINGS ? t(key) : name;
}

// Same fallback shape as orgDisplayName: a role with no copy yet renders as its raw config name
// rather than blank. Roles have no display strings today, so every role currently reads raw.
export function roleLabel(role: Role): string {
  const key = `role_name_${role}` as StringKey;
  return key in STRINGS ? t(key) : role;
}

// Fixed channels the engine names with a code (e.g. "LAndWatari") map to readable copy; everything
// dynamic — lounges, group chats, notebooks — has no entry and renders the name it was given.
export function channelLabel(name: string): string {
  const key = `channel_name_${name}` as StringKey;
  return key in STRINGS ? t(key) : name;
}

// What a poll asks, in one line: an ability's pretty name, "Arrest <player>", or the raw generic
// text. Shared by the live poll card and the start/close notice so both read the subject the same.
export function pollSubjectHeading(
  subject: PollSubject,
  players: ReadonlyMap<string, Player>,
): string {
  if ("Generic" in subject) return subject.Generic;
  if ("CivilianArrest" in subject) {
    return `Arrest ${playerLabel(slotKeyToString(subject.CivilianArrest), players)}`;
  }
  const name = Object.keys(subject.OrgAbility as Record<string, unknown>)[0] ?? "";
  return name.replace(/([a-z])([A-Z])/g, "$1 $2");
}

// The proposed ability's arguments as readable label/value pairs — the target, the flags, whatever
// the ability carries — so a voter (or a notice) sees exactly what is on the table. Empty for a
// non-ability subject; absent optional fields are skipped rather than shown blank.
export function pollSubjectArgs(
  subject: PollSubject,
  players: ReadonlyMap<string, Player>,
): { label: string; value: string }[] {
  if (!("OrgAbility" in subject)) return [];
  const beh = subject.OrgAbility as Record<string, unknown>;
  const name = Object.keys(beh)[0] ?? "";
  const raw = (beh[name] ?? {}) as Record<string, unknown>;
  const out: { label: string; value: string }[] = [];
  for (const [k, v] of Object.entries(raw)) {
    if (v === null || v === undefined) continue;
    out.push({ label: prettyArgKey(k), value: formatArgValue(v, players) });
  }
  return out;
}

// "true_name" -> "True name", "target_id" -> "Target" (the _id suffix is noise on screen).
function prettyArgKey(k: string): string {
  const s = k.replace(/_id$/, "").replace(/_/g, " ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

// Actor keys are the only object-typed args, so they are what resolves to a player name.
function formatArgValue(v: unknown, players: ReadonlyMap<string, Player>): string {
  if (typeof v === "boolean") return v ? "yes" : "no";
  if (typeof v === "object" && v !== null) {
    return playerLabel(slotKeyToString(v as never), players);
  }
  return String(v);
}

// What an ability does, sourced from the one place it is written. Empty for an ability with no copy
// yet, which the callers treat as "no description to show" rather than a blank line.
export function abilityDescription(name: AbilityName): string {
  const key = `ability_desc_${name}` as StringKey;
  return key in STRINGS ? t(key) : "";
}

// The irreversible cost of firing an ability, shown apart from the description and in the danger
// colour. Empty for the abilities that carry no such price.
export function abilityWarning(name: AbilityName): string {
  const key = `ability_warn_${name}` as StringKey;
  return key in STRINGS ? t(key) : "";
}

// A passive carrying data (a multiplier, a log kind) still reads the same base description, so the
// key comes from the variant name alone.
export function passiveDescription(passive: PassiveType): string {
  const base = typeof passive === "string" ? passive : (Object.keys(passive)[0] ?? "");
  const key = `passive_desc_${base}` as StringKey;
  return key in STRINGS ? t(key) : "";
}

// A Discord-style mention embedded in message text: `@<player:3:0>`, `@<role:Kira>`, `@<org:KK>`,
// `@<system>`. The token travels through the engine as opaque content — the client is the only
// thing that knows what an id resolves to — so parsing and resolution both live here, client-side.
// System is the one kind with no value: there is exactly one, so it needs no id.
export type Mention =
  | { kind: "player"; id: string }
  | { kind: "role"; role: Role }
  | { kind: "org"; org: OrganizationName }
  | { kind: "news_anchor" }
  | { kind: "press_conference" }
  | { kind: "system" };

// The accent for an entity, as a `var(...)` reference into the tokens in ui/theme.css. Per-entity
// tokens fall back to the kind's default, so a role/org with no colour of its own still resolves.
// Returned as a var reference rather than a resolved value so recolouring stays a theme.css edit.
export function roleColorVar(role: Role): string {
  return `var(--color-role-${role}, var(--color-role))`;
}
export function orgColorVar(org: OrganizationName): string {
  return `var(--color-org-${org}, var(--color-org))`;
}
export function mentionColorVar(mention: Mention): string {
  switch (mention.kind) {
    case "player":
      return "var(--color-mention-player)";
    case "system":
      return "var(--color-mention-system)";
    case "role":
      return roleColorVar(mention.role);
    case "org":
      return orgColorVar(mention.org);
    case "news_anchor":
      return "var(--color-news-anchor)";
    case "press_conference":
      return "var(--color-press-conference)";
  }
}

// The colour an actor's NAME renders in, by their public position: System reads System-red, the news
// anchor and press-conference members take their post's accent, and everyone else reads as an
// ordinary civilian. Anchor outranks conference outranks base. This is public status only — nothing
// gated feeds it. Takes the structural subset of a view (like ViewerIdentity) so it stays out of the
// GameView import cycle; a GameView satisfies it directly.
export function nameColorVar(
  key: string,
  status: { news_anchor: string | null; press_conf: ReadonlySet<string> },
): string {
  if (key === "System") return "var(--color-mention-system)";
  if (status.news_anchor !== null && key === status.news_anchor)
    return "var(--color-news-anchor)";
  if (status.press_conf.has(key)) return "var(--color-press-conference)";
  return "var(--color-role-Civilian)";
}

// A mention chip's colour. A player mention IS that player's name, so it follows nameColorVar and
// stays in step with how the name renders everywhere else; every other kind keeps its fixed entity
// accent. The one place mention colour and name colour are reconciled.
export function mentionChipColorVar(
  mention: Mention,
  status: { news_anchor: string | null; press_conf: ReadonlySet<string> },
): string {
  return mention.kind === "player"
    ? nameColorVar(mention.id, status)
    : mentionColorVar(mention);
}

// The colour a resolved ActorDisplay renders in — a message sender, a prosecution party. A Raw
// player follows nameColorVar so the header matches their name everywhere else; a role or org keeps
// its entity accent; System is System-red; Mysterious reads as the anonymous accent. Pairs with
// actorLabel, which resolves the text.
export function displayColorVar(
  display: ActorDisplay,
  status: { news_anchor: string | null; press_conf: ReadonlySet<string> },
): string {
  if (display === "System") return "var(--color-mention-system)";
  if (display === "Mysterious") return "var(--color-event-anonymous)";
  if ("Raw" in display) return nameColorVar(slotKeyToString(display.Raw), status);
  if ("Role" in display) return roleColorVar(display.Role);
  return "var(--color-org)";
}

// The inline style for a ping chip, given its accent var reference. The background is the same
// accent at low opacity, so one token drives both. Shared by the message renderer, the composer's
// live chips, and the death card, so a chip looks identical everywhere.
export function chipStyle(colorVar: string): string {
  return `--chip:${colorVar};color:var(--chip);background-color:color-mix(in srgb, var(--chip) 16%, transparent)`;
}

// A message body is a run of plain text and mentions. `parseMentions` is the one splitter, shared
// by rendering (segment → chip) and notification (does a mention name me?).
export type MessageSegment = { text: string } | { mention: Mention };

// Two alternatives: a kinded token whose value is `[^>]+` (so a player's `idx:version` key keeps
// its own colon — only the first colon, after the kind, is the separator), or bare `@<system>`.
const MENTION_RE = /@<(player|role|org):([^>]+)>|@<(system|news_anchor|press_conference)>/g;

export function parseMentions(content: string): MessageSegment[] {
  const segments: MessageSegment[] = [];
  let last = 0;
  for (const match of content.matchAll(MENTION_RE)) {
    const start = match.index;
    if (start > last) segments.push({ text: content.slice(last, start) });
    const [, kind, value, bare] = match;
    const mention: Mention = bare
      ? bare === "news_anchor"
        ? { kind: "news_anchor" }
        : bare === "press_conference"
          ? { kind: "press_conference" }
          : { kind: "system" }
      : kind === "player"
        ? { kind: "player", id: value }
        : kind === "role"
          ? { kind: "role", role: value as Role }
          : { kind: "org", org: value as OrganizationName };
    segments.push({ mention });
    last = start + match[0].length;
  }
  if (last < content.length) segments.push({ text: content.slice(last) });
  return segments;
}

// The token text for a mention — the inverse of one `parseMentions` segment. The composer inserts
// this into the message body, where the parser above turns it back into a chip.
export function mentionToken(mention: Mention): string {
  if (mention.kind === "player") return `@<player:${mention.id}>`;
  if (mention.kind === "role") return `@<role:${mention.role}>`;
  if (mention.kind === "org") return `@<org:${mention.org}>`;
  if (mention.kind === "news_anchor") return "@<news_anchor>";
  if (mention.kind === "press_conference") return "@<press_conference>";
  return "@<system>";
}

// The display string a mention chip shows — the `@` is dropped, the id/name becomes a real label.
export function mentionLabel(mention: Mention, players: ReadonlyMap<string, Player>): string {
  if (mention.kind === "player") return playerLabel(mention.id, players);
  if (mention.kind === "org") return orgDisplayName(mention.org);
  if (mention.kind === "role") return roleLabel(mention.role);
  if (mention.kind === "news_anchor") return t("news_anchor_label");
  if (mention.kind === "press_conference") return t("press_conference_label");
  return t("display_system");
}

// Just enough of a view to answer "does this mention name me?". A structural subset so the check
// can live here beside the parser without helpers importing GameView (which imports helpers).
export interface ViewerIdentity {
  own_key: string;
  own_role: Role | null;
  news_anchor: string | null;
  press_conf: ReadonlySet<string>;
  orgs: ReadonlyMap<string, Org>;
}

// Whether a message body mentions this viewer — by their own key, their role, or an org they are a
// member of. The one predicate behind notify-on-mention; it reuses the same parse the chips render
// from, which is the whole reason mentions are tokens and not a side channel.
export function mentionsViewer(id: ViewerIdentity, content: string): boolean {
  for (const seg of parseMentions(content)) {
    if (!("mention" in seg)) continue;
    const m = seg.mention;
    if (m.kind === "system" && id.own_key === "System") return true;
    if (m.kind === "player" && m.id === id.own_key) return true;
    if (m.kind === "role" && m.role === id.own_role) return true;
    if (m.kind === "news_anchor" && id.news_anchor !== null && id.news_anchor === id.own_key)
      return true;
    if (m.kind === "press_conference" && id.press_conf.has(id.own_key)) return true;
    if (m.kind === "org") {
      for (const org of id.orgs.values()) {
        if (org.name === m.org && org.members.has(id.own_key)) return true;
      }
    }
  }
  return false;
}

// Takes the players map rather than closing over a view, so a component can call it with whichever
// view it is rendering.
export function actorLabel(d: ActorDisplay, players: ReadonlyMap<string, Player>): string {
  if (d === "Mysterious") return t("display_mysterious");
  if (d === "System") return t("display_system");
  if ("Raw" in d) return playerLabel(slotKeyToString(d.Raw), players);
  if ("Role" in d) return d.Role;
  if ("Org" in d) return t("display_org_unknown");
  return t("display_unknown");
}

// A display's text and colour resolved against a view, shared by every surface that renders one —
// the channel feed and the log-record surfaces use the same pair, so a name reads the same whether
// it was just said live or read back out of a log.
export function actorDisplayLabel(d: ActorDisplay, view: GameView): string {
  return actorLabel(d, view.players);
}

export function actorDisplayColor(d: ActorDisplay, view: GameView): string {
  return displayColorVar(d, {
    news_anchor: view.news_anchor,
    press_conf: view.press_conf,
  });
}

// ---- prosecution phase ----

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

// The side holding the floor, or null outside the trial.
export function trialFloor(phase: ProsecutionPhaseView): "Prosecutor" | "Defense" | "Debate" | null {
  if (phase === "Voting" || "Custody" in phase) return null;
  if ("Prosecutor" in phase.Trial) return "Prosecutor";
  if ("Defense" in phase.Trial) return "Defense";
  return "Debate";
}

// Whether a non-autonomous prosecution has met the condition to leave this phase and is waiting on
// a host. Only the two phases that can be held carry the flag.
export function awaitingHost(phase: ProsecutionPhaseView): boolean {
  if (phase === "Voting") return false;
  if ("Custody" in phase) return phase.Custody.awaiting_host;
  return "Debate" in phase.Trial && phase.Trial.Debate.awaiting_host;
}

// A short label for the Prosecutions panel.
export function phaseLabel(phase: ProsecutionPhaseView): string {
  if (awaitingHost(phase)) return t("prosecution_label_awaiting_host");
  if (phase === "Voting") return t("prosecution_label_verdict_vote");
  if ("Custody" in phase) return t("prosecution_label_custody");
  if ("Debate" in phase.Trial) return t("prosecution_label_debate");

  if ("Prosecutor" in phase.Trial) {
    const side = t("prosecution_side_prosecution");
    return phase.Trial.Prosecutor === "Grace"
      ? t("prosecution_label_to_begin", { side })
      : t("prosecution_label_speaking", { side });
  }
  const side = t("prosecution_side_defense");
  return phase.Trial.Defense === "Grace"
    ? t("prosecution_label_to_begin", { side })
    : t("prosecution_label_speaking", { side });
}

// The sentence announcing a prosecution reaching this phase. Shared by news feeds and toasts so
// the wording cannot drift. `verdict` is only meaningful when ended; null there means it ended
// without one.
export function phaseAnnouncement(
  phase: ProsecutionPhaseView,
  prosecutor: string,
  defendant: string,
  ended: boolean,
  verdict: boolean | null = null,
): string {
  if (ended) {
    if (verdict === true) return t("prosecution_found_guilty", { defendant });
    if (verdict === false) return t("prosecution_acquitted", { defendant });
    return t("prosecution_ended", { defendant });
  }
  if (phase === "Voting") return t("prosecution_verdict_vote_begun", { defendant });
  if ("Custody" in phase) return t("prosecution_started", { prosecutor, defendant });
  if ("Debate" in phase.Trial) return t("prosecution_entered_debate", { defendant });

  if ("Prosecutor" in phase.Trial) {
    return phase.Trial.Prosecutor === "Grace"
      ? t("prosecution_trial_begun", { defendant })
      : t("prosecution_presents", { defendant });
  }
  return phase.Trial.Defense === "Grace"
    ? t("prosecution_defense_floor", { defendant })
    : t("prosecution_defense_presents", { defendant });
}
