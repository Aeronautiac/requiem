// Pure functions shared by the command handlers and the components. Nothing here holds state or
// reaches for a view — everything it needs is an argument, which is what lets a component call it
// without the state layer and a handler call it without the DOM.
import type {
  AbilityKey,
  AbilityName,
  ActorDisplay,
  BugKey,
  CommandRecipient,
  OrganizationName,
  PassiveKey,
  ProsecutionPhaseView,
} from "../bindings";
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { slotKeyFromString, slotKeyToString } from "../bindings";
import type { ExecError } from "../lib/protocol";
import { STRINGS, type StringKey } from "../config/strings";
import type {
  AbilityView,
  Channel,
  ChannelCategory,
  ChannelKind,
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

// ---- recipients ----

// The key of the single view a recipient targets. Undefined for a Viewport recipient, which names
// no one view — the router fans those out instead.
export function recipientToView(rec: CommandRecipient): string | undefined {
  if (rec === "System") return "System";
  if (typeof rec !== "string" && "Actor" in rec) return slotKeyToString(rec.Actor);
  return undefined;
}

export function recipientToViewport(rec: CommandRecipient): string | undefined {
  if (typeof rec !== "string" && "Viewport" in rec) return slotKeyToString(rec.Viewport);
  return undefined;
}

// The actor a command is addressed to, when it is addressed to one at all.
export function recipientToActor(rec: CommandRecipient): string | undefined {
  if (typeof rec !== "string" && "Actor" in rec) return slotKeyToString(rec.Actor);
  return undefined;
}

// ---- channel keys ----

// Bugs live in their own BugKey slot space, which can collide with real ChannelKeys, so the prefix
// keeps them separate in the channel-keyed resolve path.
export function bugChannelKey(bug_id: BugKey): string {
  return `bug:${slotKeyToString(bug_id)}`;
}

// Same reason as bugs: passives have their own slot space.
export function contactLogChannelKey(passive_id: PassiveKey): string {
  return `contacts:${slotKeyToString(passive_id)}`;
}

// The single per-viewer Notifications feed, and the one place every directed-at-you personal event
// lands. NOT News (world events), and NOT a "personal channel" (that's a real engine channel). Add
// new personal event kinds here rather than spawning a second one.
export const NOTIF_CHANNEL = "info:notifs";

// A feed you are handed rather than a room you are in. None are engine channels, so none carry
// perms, loggability or a send box — the one question each render site asks.
export function isReadOnlyKind(kind: ChannelKind): boolean {
  return kind === "Info" || kind === "Bug" || kind === "ContactLog";
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
  return {
    name,
    members: new SvelteSet<string>(),
    abilities: new SvelteMap<string, AbilityView>(),
  };
}

// Create or refresh one entry in an ability list. Shared because the same UpdateAbilityView lands
// either in a player's own list or in an org's shared one, depending only on how it was addressed.
// Takes the command's fields directly: the engine's variant is an inline struct, so bindings has
// no name for it.
export function upsert_ability(
  abilities: Map<string, AbilityView>,
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

// ---- permissions ----

// Channel permission bits, mirroring ChannelPermission in the engine.
export const PERM_SEND = 1;
export const PERM_VIEW = 2;
export const PERM_LOGGABILITY = 4;

export function hasPositivePerms(perms: number): boolean {
  return (perms & (PERM_SEND | PERM_VIEW | PERM_LOGGABILITY)) !== 0;
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
  return name.replace(/\S+/g, (word) => word[0].toUpperCase() + word.slice(1));
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
