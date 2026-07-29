<script lang="ts">
  // A plain player picker for target-taking abilities. Binds the selected player's
  // string key. The engine is the authority on valid targets (self-target, etc.), so
  // every player is offered here.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game/state.svelte";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import { playerLabel } from "../../../game/helpers.svelte";
  import type { GameState } from "../../../game/state.svelte";
  import type { UiState } from "../../../ui_state.svelte.ts";

  let {
    value = $bindable(""),
    placeholder = "Select a player",
    ids = undefined,
  }: {
    value?: string;
    placeholder?: string;
    // Optional allowlist: when set, only these player ids are offered (e.g. an org's members).
    // Omitted = every player, since the engine is the authority on valid targets.
    ids?: Iterable<string>;
  } = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // Only the players this view has been told about are offerable, which is the point: a target you
  // have never heard of is not one you get to name.
  const view = $derived(game.view_of(ui.viewer));
  const allowed = $derived(ids ? new Set(ids) : null);
  const players = $derived(
    [...view.players.entries()].filter(([id]) => !allowed || allowed.has(id)),
  );
</script>

<select
  bind:value
  class="w-full rounded-md border border-edge bg-panel px-2 py-2 text-sm text-ink"
>
  <option value="" disabled>{placeholder}</option>
  {#each players as [id] (id)}
    <option value={id}>{playerLabel(id, view.players)}</option>
  {/each}
</select>
