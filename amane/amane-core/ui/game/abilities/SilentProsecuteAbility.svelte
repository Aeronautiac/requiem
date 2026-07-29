<script lang="ts">
  import { execErrorText } from "../../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { SESSION_KEY, type SessionState } from "../../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let target = $state("");
  const flash = new Flash();

  async function accuse() {
    if (!target) {
      flash.set_error("Pick a target.");
      return;
    }
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        SilentProsecute: { target: slotKeyFromString(target) },
      }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-ink-dim">
    Name a player as wanted, with no trial and no vote. If they are wanted they die
    immediately and nothing is spent.
  </p>
  <!-- Stated plainly rather than softened: the whole shape of the ability is that the risk is
       entirely yours, and a player who does not know that cannot use it properly. -->
  <p class="text-sm text-danger">
    If they are not wanted, you are expelled from the organization and permanently barred
    from it, and the world is told your true name and which organization threw you out.
  </p>
  <PlayerSelect bind:value={target} placeholder="Target" />
  <button
    class="rounded-md bg-danger px-3 py-2 text-sm font-medium text-danger-ink hover:brightness-110"
    onclick={accuse}
  >
    Accuse
  </button>
  <FlashDisplay {flash} />
</div>
