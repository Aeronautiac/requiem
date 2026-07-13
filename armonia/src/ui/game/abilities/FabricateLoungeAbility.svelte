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

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let contactor = $state(""); // apparent initiator of the fake conversation
  let contacted = $state(""); // apparent recipient

  const flash = new Flash();

  async function fabricate() {
    if (!contactor || !contacted) {
      flash.set_error("Pick both players.");
      return;
    }
    if (contactor === contacted) {
      flash.set_error("Pick two different players.");
      return;
    }
    const err = await game.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        FabricateLounge: {
          contactor_id: slotKeyFromString(contactor),
          contacted_id: slotKeyFromString(contacted),
        },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Fabricate a private lounge between two players. Only you can see it, and you
    hold both of their identities — letting you author a conversation that never
    happened.
  </p>
  <PlayerSelect bind:value={contactor} placeholder="First player" />
  <PlayerSelect bind:value={contacted} placeholder="Second player" />
  <button
    class="rounded-md bg-amber-600 px-3 py-2 text-sm font-medium text-white hover:bg-amber-500"
    onclick={fabricate}
  >
    Fabricate lounge
  </button>
  <FlashDisplay {flash} />
</div>
