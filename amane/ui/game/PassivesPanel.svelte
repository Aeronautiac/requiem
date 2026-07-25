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
  import * as Dialog from "$lib/components/ui/dialog";

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

<Dialog.Root bind:open>
  <Dialog.Trigger
    class="h-8 rounded-md border border-neutral-700 bg-neutral-900 px-3 text-sm text-neutral-200 hover:bg-neutral-800"
  >
    Passives
  </Dialog.Trigger>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>Passives</Dialog.Title>
    </Dialog.Header>
    <div class="flex flex-col gap-1.5">
      {#each passives as [id, pv] (id)}
        <div
          class="rounded-md border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm text-neutral-200"
        >
          {prettyPassive(pv.type)}
        </div>
      {/each}
      {#if passives.length === 0}
        <p class="py-2 text-sm text-neutral-600">No passives.</p>
      {/if}
    </div>
  </Dialog.Content>
</Dialog.Root>
