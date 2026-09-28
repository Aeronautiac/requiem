// Ability listing and request-building shared by the abilities menu and its forms. Plain
// functions, no React, so a headless harness can drive an ability the same way the menu does.
import type { AbilityBehaviour, AbilityName, ActionRequest, OrgAbility } from "../bindings.ts";
import { slotKeyFromString } from "../bindings.ts";
import type { View } from "../game/view.ts";
import { viewActor } from "./session.ts";

// Surfaced through a dedicated widget elsewhere, so the generic menu leaves them out rather than
// showing the same ability twice.
export const EXCLUDED_ABILITIES: ReadonlySet<AbilityName> = new Set<AbilityName>([
  "Contact", // Players widget
  "CreateGroupchat", // Channels widget (Group Chats category)
]);

export type ListedAbility = {
  id: string;
  name: AbilityName;
  successUsages: number;
  failureUsages: number;
  resets: number;
  baseReset: number;
  unlimited: boolean;
  // Org abilities only: the static gates on firing this. Undefined for a personal ability.
  requirements?: OrgAbility;
};

// The abilities a menu offers: an org's shared set, or the viewer's own, minus what's surfaced
// elsewhere. Order follows the source map's insertion order, which is arrival order — the same
// order the old menu showed them in.
export function listAbilities(view: View, orgId?: string): ListedAbility[] {
  const source = orgId ? view.orgs.get(orgId)?.abilities : view.abilities;
  const out: ListedAbility[] = [];
  for (const [id, av] of source ?? []) {
    if (EXCLUDED_ABILITIES.has(av.name)) continue;
    out.push({
      id,
      name: av.name,
      successUsages: av.success_usages_remaining,
      failureUsages: av.failure_usages_remaining,
      resets: av.iterations_to_reset,
      baseReset: av.base_reset,
      unlimited: av.unlimited,
      requirements: av.requirements,
    });
  }
  return out;
}

// "AnonymousContact" -> "Anonymous Contact".
export function prettyAbility(name: AbilityName): string {
  return name.replace(/([a-z])([A-Z])/g, "$1 $2");
}

// Usage counts are split by outcome: a pool may only be spent on success, only on failure, or
// both. Usable as long as some outcome still has charges — or there are no pools at all, which
// means no restriction.
export function abilityUsable(ab: Pick<ListedAbility, "unlimited" | "successUsages" | "failureUsages">): boolean {
  return ab.unlimited || ab.successUsages > 0 || ab.failureUsages > 0;
}

// The engine decides whether an org ability fires immediately or opens a vote; the request shape
// is the only thing that differs between a personal and an org use.
export function abilityRequest(
  viewer: string,
  abilityId: string,
  orgId: string | undefined,
  behaviour: AbilityBehaviour,
  timestamp: number,
): ActionRequest {
  return {
    actor: viewActor(viewer),
    timestamp,
    payload: orgId
      ? {
          UseOrgAbility: {
            org_id: slotKeyFromString(orgId),
            ability_id: slotKeyFromString(abilityId),
            ability_args: behaviour,
          },
        }
      : {
          UseAbility: {
            ability_id: slotKeyFromString(abilityId),
            ability_args: behaviour,
          },
        },
  };
}
