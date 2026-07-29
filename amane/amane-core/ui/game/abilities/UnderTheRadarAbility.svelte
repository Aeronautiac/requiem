<script lang="ts">
  import { execErrorText } from "../../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { SESSION_KEY, type SessionState } from "../../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const flash = new Flash();

  async function go_dark() {
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, { UnderTheRadar: {} }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
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
