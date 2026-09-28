// Pure functions that turn state into words. Nothing here holds state or reaches for a view —
// everything it needs is an argument, which is what lets a component call it without the state
// layer and a handler call it without the DOM.
import type {
  AbilityName,
  ActorDisplay,
  OrganizationName,
  PassiveType,
  PollSubject,
  PrivilegeSet,
  ProsecutionPhaseView,
  Role,
  Statuses,
} from "./bindings.ts";
import { slotKeyFromString, slotKeyToString, StatusFlag } from "./bindings.ts";
import type { ExecError } from "./host.ts";
import { STRINGS, type StringKey } from "./strings.ts";
import { awaitingHost } from "./game/prosecution.ts";
import type { ChannelPerms, Org, Player } from "./game/types.ts";

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
    case "server": {
      const key = `server_${error.code}` as StringKey;
      return key in STRINGS ? t(key) : error.code;
    }
    case "network":
      return t("exec_network");
  }
}

// Resolve a copy key, filling `{name}` placeholders. The only way a string reaches the screen.
export function t(key: StringKey, vars?: Record<string, string | number>): string {
  // `keyof STRINGS` includes platform_splashes (an array, never looked up by name), so the lookup
  // is narrowed to the string-copy case here.
  const template = STRINGS[key] as string;
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in vars ? String(vars[name]) : whole,
  );
}

// ---- permissions ----

// What one name may do, for display beside it. Loggability control is deliberately absent: it is
// about the channel rather than about standing in it, and it belongs to the toggle it drives.
export function permsLabel(perms: ChannelPerms): string {
  const parts: string[] = [];
  if (perms.read) parts.push("read");
  if (perms.send) parts.push("send");
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

// What a key permits, in one line: its administrative standing, then who it may act as. Players
// are named from `players` where a game is at hand; outside one (the account's saved keys) they
// read by slot.
export function privilegesLabel(
  privileges: PrivilegeSet,
  players: ReadonlyMap<string, Player> = new Map(),
): string {
  const caps = privileges.capabilities;
  const standing = caps.includes("Supervise")
    ? t("key_supervisor")
    : caps.includes("Administer")
      ? t("key_admin")
      : t("key_player");
  const actors = privileges.actors;
  const scope =
    actors === "All"
      ? t("key_every_actor")
      : actors.Only.length === 0
        ? t("key_no_actors")
        : actors.Only.map((a) => playerLabel(slotKeyToString(a), players)).join(", ");
  return `${standing} · ${scope}`;
}

// Falls back to the raw config code, so an org added to the engine before the copy exists still
// renders as something rather than blank.
export function orgDisplayName(name: OrganizationName): string {
  const key = `org_name_${name}` as StringKey;
  return key in STRINGS ? t(key) : name;
}

// Same fallback shape as orgDisplayName: a role with no copy yet renders as its raw config name
// rather than blank. Most roles have a display string (strings.ts, role_name_*); the raw name is
// only the fallback until one is written.
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

// ---- references: anything a name can stand for ----

// What a name on screen refers to, however it arrived: an actor key (a roster row, a poll opener),
// an engine ActorDisplay (a message sender, a prosecution party), or a mention token in text. Each
// of those is converted into a Ref once, and everything after — the words (`refLabel`), the colour
// (the UI's `refColor`), the markup — answers for all three at once.
export type Ref =
  | { kind: "player"; id: string }
  | { kind: "role"; role: Role }
  | { kind: "org"; org: OrganizationName }
  | { kind: "news_anchor" }
  | { kind: "press_conference" }
  | { kind: "system" }
  // A name deliberately withheld (an anonymous display).
  | { kind: "mysterious" }
  // A key or org this view was never told about.
  | { kind: "unknown" };

// A Discord-style mention embedded in message text: `@<player:3:0>`, `@<role:Kira>`, `@<org:KK>`,
// `@<system>`. The token travels through the engine as opaque content — the client is the only
// thing that knows what an id resolves to — so parsing and resolution both live here, client-side.
// A mention is a Ref that can be written as a token; the two kinds that can't be named are left out.
export type Mention = Exclude<Ref, { kind: "mysterious" } | { kind: "unknown" }>;

// An engine display as a Ref. An org display carries the org's key, which resolves to a name only
// for a view that has been told about that org.
export function refFromDisplay(display: ActorDisplay, orgs: ReadonlyMap<string, Org>): Ref {
  if (display === "Mysterious") return { kind: "mysterious" };
  if (display === "System") return { kind: "system" };
  if ("Raw" in display) return { kind: "player", id: slotKeyToString(display.Raw) };
  if ("Role" in display) return { kind: "role", role: display.Role };
  const org = orgs.get(slotKeyToString(display.Org));
  return org ? { kind: "org", org: org.name } : { kind: "unknown" };
}

// An actor key as a Ref: a player or an org this view holds, or System.
export function refFromKey(
  key: string,
  players: ReadonlyMap<string, Player>,
  orgs: ReadonlyMap<string, Org>,
): Ref {
  if (key === "System") return { kind: "system" };
  if (players.has(key)) return { kind: "player", id: key };
  const org = orgs.get(key);
  return org ? { kind: "org", org: org.name } : { kind: "unknown" };
}

// The words a Ref reads as. The one label function for every name on screen.
export function refLabel(ref: Ref, players: ReadonlyMap<string, Player>): string {
  switch (ref.kind) {
    case "player":
      return playerLabel(ref.id, players);
    case "role":
      return roleLabel(ref.role);
    case "org":
      return orgDisplayName(ref.org);
    case "news_anchor":
      return t("news_anchor_label");
    case "press_conference":
      return t("press_conference_label");
    case "system":
      return t("display_system");
    case "mysterious":
      return t("display_mysterious");
    case "unknown":
      return t("display_unknown");
  }
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

// Just enough of a view to answer "does this mention name me?". A structural subset so the check
// can live here without needing a whole View.
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

// ---- log records ----

// The name a log record shows in a sidebar or header.
export function logDumpLabel(
  key: string,
  players: ReadonlyMap<string, Player>,
  orgs: ReadonlyMap<string, Org>,
): string {
  if (key.startsWith("autopsy:")) {
    const target = key.slice("autopsy:".length);
    return `Autopsy: ${refLabel(refFromKey(target, players, orgs), players)}`;
  }
  if (key.startsWith("tapin:")) {
    return `Tap In: ${key.slice("tapin:".length)}`;
  }
  return "Record";
}

// ---- prosecution phase ----

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

// Which sentence announces a prosecution reaching this phase. The one decision behind the news
// feed and the toasts, so their wording cannot drift: each fills the same template, with
// `{prosecutor}` and `{defendant}` as plain names (a toast, `t(key, vars)`) or as rendered names (the
// feed). `verdict` is only meaningful when ended; null there means it ended without one.
export function phaseAnnouncementKey(
  phase: ProsecutionPhaseView,
  ended: boolean,
  verdict: boolean | null = null,
): StringKey {
  if (ended) {
    if (verdict === true) return "prosecution_found_guilty";
    if (verdict === false) return "prosecution_acquitted";
    return "prosecution_ended";
  }
  if (phase === "Voting") return "prosecution_verdict_vote_begun";
  if ("Custody" in phase) return "prosecution_started";
  if ("Debate" in phase.Trial) return "prosecution_entered_debate";
  if ("Prosecutor" in phase.Trial) {
    return phase.Trial.Prosecutor === "Grace" ? "prosecution_trial_begun" : "prosecution_presents";
  }
  return phase.Trial.Defense === "Grace" ? "prosecution_defense_floor" : "prosecution_defense_presents";
}

// ---- time ----

// A timestamp on the wire is GAME time: the sandbox counts up from 0, so a moment is how far into
// the game it happened, not a point on the wall clock. The raw duration of a game day can change
// mid-game, but these raw units never do -- so this renders the raw elapsed game time with explicit
// units. There is no real-world wall-time counterpart: with time travel the game clock is untethered
// from the wall clock, so a wall date has no meaning attached to a game moment.
export function formatTime(gameMs: number): string {
  const totalSec = Math.max(0, Math.floor(gameMs / 1000));
  const d = Math.floor(totalSec / 86400);
  const h = Math.floor((totalSec % 86400) / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  const days = d > 0 ? `${d}d:` : "";
  return `${days}${h}h:${String(m).padStart(2, "0")}m:${String(s).padStart(2, "0")}s`;
}

// Engine Time is unix ms, so durations like a notebook-write delay arrive in ms. Shows the largest
// one or two non-zero leading units: "2m 30s", "1h 1m", "800ms".
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const units: [number, string][] = [
    [86400000, "d"],
    [3600000, "h"],
    [60000, "m"],
    [1000, "s"],
  ];
  const parts: string[] = [];
  let rem = ms;
  for (const [size, label] of units) {
    if (rem >= size) {
      const val = Math.floor(rem / size);
      parts.push(`${val}${label}`);
      rem -= val * size;
      if (parts.length === 2) break;
    } else if (parts.length > 0) {
      break; // stop at the first gap once we've started, keeping it to leading units
    }
  }
  return parts.join(" ");
}
