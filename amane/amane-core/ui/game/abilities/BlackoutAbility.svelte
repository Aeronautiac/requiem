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
      useAbilityRequest(ui.viewer, abilityId, orgId, { Blackout: {} }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <button
    class="rounded-md bg-danger px-3 py-2 text-sm font-medium text-danger-ink hover:brightness-110"
    onclick={go_dark}
  >
    Cut the lights
  </button>
  <FlashDisplay {flash} />
</div>
