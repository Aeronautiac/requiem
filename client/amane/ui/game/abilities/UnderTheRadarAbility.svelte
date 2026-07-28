<script lang="ts">
  import { getContext } from "svelte";
  import { CLIENT_KEY, type ClientState } from "../../../client.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const flash = new Flash();

  async function go_dark() {
    const err = await client.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, { UnderTheRadar: {} }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Go off the record for the rest of the iteration. Nothing you say is logged, no bug
    relays you, and the contacts you open leave no trace in anyone's contact log.
  </p>
  <p class="text-sm text-neutral-500">
    It does not make you inaudible: the people in a room still hear what you say there.
  </p>
  <button
    class="rounded-md bg-neutral-700 px-3 py-2 text-sm font-medium text-white hover:bg-neutral-600"
    onclick={go_dark}
  >
    Go under the radar
  </button>
  <FlashDisplay {flash} />
</div>
