<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let target = $state("");
  const flash = new Flash();

  async function grant() {
    if (!target) {
      flash.set_error("Pick a target.");
      return;
    }
    const err = await game.dispatch(
      useAbilityRequest(ui.viewer, abilityId, {
        Ipp: { target: slotKeyFromString(target) },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Grant IPP to a player — strengthened presence and immunity from being written.
  </p>
  <PlayerSelect bind:value={target} placeholder="Target" />
  <button
    class="rounded-md bg-emerald-600 px-3 py-2 text-sm font-medium text-white hover:bg-emerald-500"
    onclick={grant}
  >
    Grant IPP
  </button>
  <FlashDisplay {flash} />
</div>
