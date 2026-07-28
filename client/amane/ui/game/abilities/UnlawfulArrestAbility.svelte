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

  async function arrest() {
    if (!target) {
      flash.set_error("Pick someone to arrest.");
      return;
    }
    const err = await client.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        UnlawfulArrest: { target: slotKeyFromString(target) },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Jail a player immediately, with no vote. The world sees the imprisonment but never
    learns who ordered it.
  </p>
  <PlayerSelect bind:value={target} placeholder="Arrest target" />
  <button
    class="rounded-md bg-orange-600 px-3 py-2 text-sm font-medium text-white hover:bg-orange-500"
    onclick={arrest}
  >
    Arrest
  </button>
  <FlashDisplay {flash} />
</div>
