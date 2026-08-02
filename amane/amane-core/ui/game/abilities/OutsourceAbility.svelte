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

  let invitee = $state("");
  let defendant = $state("");
  const flash = new Flash();

  async function run() {
    if (!invitee || !defendant) {
      flash.set_error("Pick who prosecutes and who is prosecuted.");
      return;
    }
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        Outsource: {
          invitee: slotKeyFromString(invitee),
          defendant: slotKeyFromString(defendant),
        },
      }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Delegate a prosecution: pull a player into the org and set them prosecuting someone on the
    org's behalf. Draws the org's invite and prosecution pools.
  </p>
  <span class="text-xs uppercase tracking-wide text-neutral-500">Prosecutor</span>
  <PlayerSelect bind:value={invitee} placeholder="Who to bring in and set prosecuting" />
  <span class="text-xs uppercase tracking-wide text-neutral-500">Defendant</span>
  <PlayerSelect bind:value={defendant} placeholder="Who they prosecute" />
  <button
    class="rounded-md bg-amber-600 px-3 py-2 text-sm font-medium text-white hover:bg-amber-500"
    onclick={run}
  >
    Outsource prosecution
  </button>
  <FlashDisplay {flash} />
</div>
