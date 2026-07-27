<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY, playerLabel } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import Select from "../kit/Select.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  // Admin first, then player views sorted by slot index. System is an internal view (not a real
  // player) and is what "Admin" selects, so it isn't offered under its own name.
  const options = $derived([
    { value: "Admin", label: "Admin" },
    ...Array.from(game.views.keys())
      .filter((k) => k !== "System")
      .sort((a, b) => parseInt(a) - parseInt(b))
      .map((key) => ({ value: key, label: playerLabel(key, game.players) })),
  ]);
</script>

<Select bind:value={ui.viewer} {options} class="h-8" />
