<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import * as Select from "$lib/components/ui/select";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  // Player views only, sorted by slot index. Base and System are internal views (not
  // real players), so they're kept out of the picker; Admin is offered separately below.
  const viewers = $derived(
    Array.from(game.views.keys())
      .filter((k) => k !== "Base" && k !== "System")
      .sort((a, b) => parseInt(a) - parseInt(b)),
  );

  function label(key: string): string {
    return game.players.get(key)?.display_name ?? key;
  }
</script>

<Select.Root type="single" bind:value={ui.viewer}>
  <Select.Trigger class="h-8 text-sm">{label(ui.viewer)}</Select.Trigger>
  <Select.Content>
    <Select.Item value={"Admin"}>{"Admin"}</Select.Item>
    {#each viewers as viewer (viewer)}
      <Select.Item value={viewer}>{label(viewer)}</Select.Item>
    {/each}
  </Select.Content>
</Select.Root>
