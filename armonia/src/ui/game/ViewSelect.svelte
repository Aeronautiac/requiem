<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import * as Select from "$lib/components/ui/select";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  // Admin first, then players sorted by slot index
  const viewers = $derived(
    Array.from(game.views.keys()).sort((a, b) => {
      if (a === "Admin") return -1;
      if (b === "Admin") return 1;
      return parseInt(a) - parseInt(b);
    }),
  );

  function label(key: string): string {
    return key === "Admin"
      ? "Admin"
      : (game.players.get(key)?.display_name ?? key);
  }
</script>

<Select.Root type="single" bind:value={ui.viewer}>
  <Select.Trigger class="h-8 text-sm">{label(ui.viewer)}</Select.Trigger>
  <Select.Content>
    {#each viewers as viewer (viewer)}
      <Select.Item value={viewer}>{label(viewer)}</Select.Item>
    {/each}
  </Select.Content>
</Select.Root>
