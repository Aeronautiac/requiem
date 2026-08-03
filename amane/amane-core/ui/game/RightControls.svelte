<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { TopPanel } from "../../ui_state.svelte.ts";
  import OrgPanel from "./OrgPanel.svelte";

  // The rail's action row: the org menu, plus the two toggles that raise the widget strip under the
  // channel header. Sits above the channel-members list.
  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const view = $derived(game.view_of(ui.viewer));

  const poll_count = $derived(
    [...view.polls.values()].filter((p) => !p.outcome).length,
  );
  const prosecution_count = $derived(view.prosecutions.size);

  const toggles: { panel: TopPanel; label: string; count: number }[] = $derived([
    { panel: "polls", label: "Polls", count: poll_count },
    { panel: "prosecutions", label: "Prosecutions", count: prosecution_count },
  ]);
</script>

<div class="flex flex-col gap-1 border-b border-neutral-800 p-2">
  <OrgPanel />

  {#each toggles as t (t.panel)}
    <button
      class="flex items-center gap-2 border px-3 py-1.5 text-xs font-medium {ui.top_panel ===
      t.panel
        ? 'border-neutral-600 bg-neutral-800 text-neutral-100'
        : 'border-edge bg-panel text-ink hover:bg-raised'}"
      onclick={() => ui.toggle_panel(t.panel)}
    >
      {t.label}
      {#if t.count > 0}
        <span class="ml-auto bg-neutral-950 px-1.5 text-[0.65rem] text-neutral-400">
          {t.count}
        </span>
      {/if}
    </button>
  {/each}
</div>
