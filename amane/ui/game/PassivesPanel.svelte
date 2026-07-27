<script lang="ts">
  // Read-only list of the current viewer's passives, opened from a button beside Abilities.
  // Passives aren't used (no charges), just observed — some carry data (e.g. a vote
  // amplification multiplier), which is shown inline.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { PassiveType } from "../../bindings";
  import Dialog from "../kit/Dialog.svelte";
  import Button from "../kit/Button.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let open = $state(false);

  const passives = $derived([
    ...(game.views.get(ui.viewer)?.passives.entries() ?? []),
  ]);

  function prettyPassive(p: PassiveType): string {
    // string variants: split camelCase ("CustodyBugReceiver" -> "Custody Bug Receiver")
    if (typeof p === "string") return p.replace(/([a-z])([A-Z])/g, "$1 $2");
    if ("VoteAmplification" in p)
      return `Vote Amplification (×${p.VoteAmplification.multiplier})`;
    if ("ContactLogs" in p) return `Contact Logs (${p.ContactLogs})`;
    return "Passive";
  }
</script>

<Button variant="ghost" size="sm" onclick={() => (open = true)}>Passives</Button>

<Dialog bind:open title="Passives" class="max-w-sm">
  <div class="flex flex-col gap-1.5">
    {#each passives as [id, pv] (id)}
      <div class="rounded-md border border-edge bg-raised px-3 py-2 text-sm text-ink">
        {prettyPassive(pv.type)}
      </div>
    {/each}
    {#if passives.length === 0}
      <p class="py-2 text-sm text-ink-dim">No passives.</p>
    {/if}
  </div>
</Dialog>
