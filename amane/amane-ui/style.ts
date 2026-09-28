// Everything that decides a colour. Colours are `var(...)` references into theme.css, never
// literals, so recolouring is a theme edit.
import type { OrganizationName, Role, Statuses } from "amane-client/bindings.ts";
import { StatusFlag } from "amane-client/bindings.ts";
import type { View } from "amane-client/game/view.ts";
import type { Ref } from "amane-client/text.ts";
import type { CSSProperties } from "react";

export function roleColorVar(role: Role): string {
  return `var(--color-role-${role}, var(--color-role))`;
}

export function orgColorVar(org: OrganizationName): string {
  return `var(--color-org-${org}, var(--color-org))`;
}

// The colour a name renders in, whatever it refers to: the partner of `refLabel`. A player follows
// `nameColorVar`, so their name reads the same in a message header, a mention and a roster; every
// other kind has its entity's accent.
export function refColor(ref: Ref, view: View): string {
  switch (ref.kind) {
    case "player":
      return nameColorVar(ref.id, view);
    case "role":
      return roleColorVar(ref.role);
    case "org":
      return orgColorVar(ref.org);
    case "news_anchor":
      return "var(--color-news-anchor)";
    case "press_conference":
      return "var(--color-press-conference)";
    case "system":
      return "var(--color-mention-system)";
    case "mysterious":
      return "var(--color-event-anonymous)";
    case "unknown":
      return "var(--color-ink-dim)";
  }
}

// The accent a public-status badge renders in, by its label (see statusLabels).
export function statusAccent(status: string): string {
  switch (status) {
    case "dead":
      return "var(--color-status-dead)";
    case "missing":
      return "var(--color-status-missing)";
    case "custody":
      return "var(--color-status-custody)";
    case "incarcerated":
      return "var(--color-status-incarcerated)";
    case "kidnapped":
      return "var(--color-status-kidnapped)";
    case "bugged":
      return "var(--color-status-bugged)";
    case "ipp":
      return "var(--color-status-ipp)";
    default:
      return "var(--color-status-default)";
  }
}

// Text in the accent over the same accent at low opacity: how every badge and chip tints itself.
export function tint(colorVar: string): CSSProperties {
  return { color: colorVar, backgroundColor: `color-mix(in srgb, ${colorVar} 16%, transparent)` };
}

// The colour a name takes from a presence status, worst first, or null for the ordinary colour.
export function statusNameColor(status: Statuses): string | null {
  if (status & StatusFlag.Dead) return "var(--color-status-dead)";
  if (status & StatusFlag.Kidnapped) return "var(--color-status-kidnapped)";
  if (status & StatusFlag.Custody) return "var(--color-status-custody)";
  if (status & StatusFlag.Incarcerated) return "var(--color-status-incarcerated)";
  if (status & StatusFlag.Missing) return "var(--color-status-missing)";
  return null;
}

// The colour a player's NAME renders in, from public facts only: a presence status first, then
// the news anchor, the press conference, and otherwise an ordinary civilian. Per view, since the
// same key can hold a post in one view's knowledge and not another's.
function nameColorVar(key: string, view: View): string {
  const status = view.actor_statuses.get(key);
  if (status !== undefined) {
    const color = statusNameColor(status);
    if (color) return color;
  }
  if (view.news_anchor !== null && key === view.news_anchor) return "var(--color-news-anchor)";
  if (view.press_conf.has(key)) return "var(--color-press-conference)";
  return "var(--color-role-Civilian)";
}
