<script lang="ts">
  // A plain player picker for target-taking abilities. Binds the selected player's
  // string key. The engine is the authority on valid targets (self-target, etc.), so
  // every player is offered here.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";

  let {
    value = $bindable(""),
    placeholder = "Select a player",
  }: { value?: string; placeholder?: string } = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const players = $derived([...game.players.entries()]);
</script>

<select
  bind:value
  class="w-full rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
>
  <option value="" disabled>{placeholder}</option>
  {#each players as [id, p] (id)}
    <option value={id}>{p.display_name}</option>
  {/each}
</select>
