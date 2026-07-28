<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../../client.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let lounge = $state("");
  const flash = new Flash();

  // Every lounge the viewer is in, by the channel it shows up as. Only a Basic lounge actually
  // qualifies, but the client is not told which variant a lounge is — an anonymous line looks
  // like any other from here — so all of them are offered and the engine rejects the rest.
  const lounges = $derived.by(() => {
    const view = game.views.get(ui.viewer);
    const out: { lounge_id: string; name: string }[] = [];
    for (const [channel_key, channel] of game.channels) {
      if (channel.category !== "Lounge") continue;
      if (!view?.channel_views.get(channel_key)?.perms.had_positive) continue;
      const lounge_id = game.lounge_key_for_channel(channel_key);
      if (lounge_id) out.push({ lounge_id, name: channel.name });
    }
    return out;
  });

  async function connect() {
    if (!lounge) {
      flash.set_error("Pick a line to reach through.");
      return;
    }
    const err = await client.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        KiraConnection: { lounge: slotKeyFromString(lounge) },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Reach for Kira down a line you already have. Only a direct, non-anonymous line can
    establish who is really on the other end.
  </p>
  <p class="text-sm text-neutral-500">
    The attempt lands in that lounge either way, naming you and saying whether it worked.
  </p>
  <select
    bind:value={lounge}
    class="w-full rounded-md border border-edge bg-panel px-2 py-2 text-sm text-ink"
  >
    <option value="" disabled>Select a line</option>
    {#each lounges as l (l.lounge_id)}
      <option value={l.lounge_id}>{l.name}</option>
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
