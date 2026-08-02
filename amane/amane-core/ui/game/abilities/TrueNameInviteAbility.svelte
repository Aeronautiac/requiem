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
  let trueName = $state("");
  const flash = new Flash();

  async function invite() {
    if (!target) {
      flash.set_error("Pick who to invite.");
      return;
    }
    if (!trueName.trim()) {
      flash.set_error("Guess their true name.");
      return;
    }
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        TrueNameInvite: { target: slotKeyFromString(target), true_name: trueName.trim() },
      }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Invite a player by guessing their true name. Get it right and they join the org and their
    name is revealed to the members. Get it wrong and you have spent the attempt for nothing.
  </p>
  <PlayerSelect bind:value={target} placeholder="Who to invite" />
  <input
    bind:value={trueName}
    placeholder="Their true name"
    class="w-full rounded-md border border-edge bg-panel px-2 py-2 text-sm text-ink"
  />
  <button
    class="rounded-md bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-500"
    onclick={invite}
  >
    Send invite
  </button>
  <FlashDisplay {flash} />
</div>
