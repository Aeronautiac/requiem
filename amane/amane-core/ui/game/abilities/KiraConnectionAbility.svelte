<script lang="ts">
  import { channelLabel, execErrorText } from "../../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game/state.svelte";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  let lounge = $state("");
  const flash = new Flash();

  // Every lounge the viewer is in, by the channel it shows up as. Only a Basic lounge actually
  // qualifies, but the client is not told which variant a lounge is — an anonymous line looks
  // like any other from here — so all of them are offered and the engine rejects the rest.
  const lounges = $derived.by(() => {
    const out: { lounge_id: string; name: string }[] = [];
    for (const [channel_key, channel] of view.channels) {
      if (channel.category !== "Lounge") continue;
      const lounge_id = view.lounge_of(channel_key);
      if (lounge_id) out.push({ lounge_id, name: channel.name });
    }
    return out;
  });

  async function connect() {
    if (!lounge) {
      flash.set_error("Pick a line to reach through.");
      return;
    }
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        KiraConnection: { lounge: slotKeyFromString(lounge) },
      }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <select
    bind:value={lounge}
    class="w-full rounded-md border border-edge bg-panel px-2 py-2 text-sm text-ink"
  >
    <option value="" disabled>Select a line</option>
    {#each lounges as l (l.lounge_id)}
      <option value={l.lounge_id}>{channelLabel(l.name)}</option>
    {/each}
  </select>
  <button
    class="rounded-md bg-red-600 px-3 py-2 text-sm font-medium text-white hover:bg-red-500"
    onclick={connect}
  >
    Reach for Kira
  </button>
  <FlashDisplay {flash} />
</div>
