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

  // A raw number, not a picker: the whole ability is guessing which contact channel a number
  // belongs to, so there is nothing to choose from. A channel you could pick is one you are
  // already in.
  let contact_id = $state<number | null>(null);
  const flash = new Flash();

  async function tap() {
    if (contact_id === null || contact_id < 0 || !Number.isInteger(contact_id)) {
      flash.set_error("Enter a contact number.");
      return;
    }
    const err = await client.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        TapIn: { contact_id },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Read a contact channel's record by guessing its number. Lounges and group chats are numbered
    in one running sequence — you tap what you can work out from what you already know.
  </p>
  <p class="text-sm text-neutral-500">
    Wrong guesses are limited, and the channel is told it was read — though never by whom.
  </p>
  <input
    type="number"
    min="0"
    step="1"
    bind:value={contact_id}
    placeholder="Contact number"
    class="w-full rounded-md border border-edge bg-panel px-2 py-2 text-sm text-ink"
  />
  <button
    class="rounded-md bg-amber-600 px-3 py-2 text-sm font-medium text-white hover:bg-amber-500"
    onclick={tap}
  >
    Tap in
  </button>
  <FlashDisplay {flash} />
</div>
