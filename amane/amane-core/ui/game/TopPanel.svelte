<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { PollData, PollView } from "../../game/types";
  import PollCard from "./PollCard.svelte";
  import ProsecutionCard from "./ProsecutionCard.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const view = $derived(game.view_of(ui.viewer));

  // Active polls only. A resolved poll keeps its entry (for a late viewer's replay), so `outcome`
  // is the liveness test — same rule the old sidebar used.
  const polls = $derived.by(() => {
    const out: { id: string; data: PollData; pollView: PollView | null; frozen: boolean }[] = [];
    for (const [id, data] of view.polls) {
      if (data.outcome) continue;
      out.push({
        id,
        data,
        pollView: view.poll_views.get(id) ?? null,
        frozen: view.frozen(view.poll_viewport(id)),
      });
    }
    return out;
  });

  const prosecutions = $derived([...view.prosecutions.entries()]);
</script>

<div class="flex flex-col border-b border-neutral-800 bg-neutral-950">
  <div class="flex items-center gap-2 px-3 py-1">
    <span class="text-xs font-medium uppercase tracking-wide text-neutral-400">
      {ui.top_panel === "polls" ? "Polls" : "Prosecutions"}
    </span>
    <span class="text-[0.65rem] text-neutral-600">
      {ui.top_panel === "polls" ? polls.length : prosecutions.length}
    </span>
    <button
      class="ml-auto px-1.5 text-neutral-500 hover:text-neutral-200"
      aria-label="Close panel"
      onclick={() => (ui.top_panel = null)}
    >
      ✕
    </button>
  </div>

  <div class="flex gap-2 overflow-x-auto px-3 pb-2">
    {#if ui.top_panel === "polls"}
      {#each polls as p (p.id)}
        <PollCard
          id={p.id}
          data={p.data}
          pollView={p.pollView}
          frozen={p.frozen}
          variant="panel"
        />
      {/each}
      {#if polls.length === 0}
        <p class="py-2 text-xs text-neutral-600">No active votes.</p>
      {/if}
    {:else}
      {#each prosecutions as [id, data] (id)}
        <ProsecutionCard {id} {data} />
      {/each}
      {#if prosecutions.length === 0}
        <p class="py-2 text-xs text-neutral-600">No active prosecutions.</p>
      {/if}
    {/if}
  </div>
</div>
