<script lang="ts">
  import { getContext } from "svelte";
  import { CLIENT_KEY, type ClientState } from "../../../client.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let target = $state("");
  const flash = new Flash();

  async function reroll() {
    if (!target) {
      flash.set_error("Pick whose name to reroll.");
      return;
    }
    const err = await client.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        // The server draws the new name and replaces this before the engine sees it, the same
        // way it replaces the timestamp — naming yourself is not a thing a client gets to do.
        // The field is carried anyway because the action has to be self-contained to replay.
        TrueNameReroll: { target: slotKeyFromString(target), true_name: "" },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Give a player a new true name, drawn by the server. Anyone holding the old one is holding
    something worthless — and you will not be told the new one.
  </p>
  <p class="text-sm text-neutral-500">Single use.</p>
  <PlayerSelect bind:value={target} placeholder="Whose name to reroll" />
  <button
    class="rounded-md bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-500"
    onclick={reroll}
  >
    Reroll name
  </button>
  <FlashDisplay {flash} />
</div>
