import { slotKeyToString } from "../bindings.ts";
import type { AbilityKey, AbilityName } from "../bindings.ts";
import type { AbilityView } from "./types.ts";

// Create or refresh one entry in an ability list. Shared because the same UpdateAbilityView lands
// either in a player's own list or in an org's shared one, depending only on how it was addressed.
// Takes the command's fields directly: the engine's variant is an inline struct, so bindings has no
// name for it.
export function upsertAbility(
  abilities: Map<string, AbilityView>,
  payload: {
    ability_id: AbilityKey;
    ability_name: AbilityName;
    success_usages_remaining: number;
    failure_usages_remaining: number;
    iterations_to_reset: number;
    base_reset: number;
    unlimited: boolean;
  },
) {
  const id = slotKeyToString(payload.ability_id);
  const existing = abilities.get(id);
  if (existing) {
    existing.success_usages_remaining = payload.success_usages_remaining;
    existing.failure_usages_remaining = payload.failure_usages_remaining;
    existing.iterations_to_reset = payload.iterations_to_reset;
    existing.base_reset = payload.base_reset;
    existing.unlimited = payload.unlimited;
    return;
  }
  abilities.set(id, {
    name: payload.ability_name,
    success_usages_remaining: payload.success_usages_remaining,
    failure_usages_remaining: payload.failure_usages_remaining,
    iterations_to_reset: payload.iterations_to_reset,
    base_reset: payload.base_reset,
    unlimited: payload.unlimited,
  });
}
