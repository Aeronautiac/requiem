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
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let sacrifice = $state("");
  let name_target = $state("");
  const flash = new Flash();

  // Only an OG member can be spent, so offer the org's roster rather than every player. The
  // engine still checks it — this is UX. Undefined (no org) leaves PlayerSelect unfiltered.
  const members = $derived(orgId ? game.orgs.get(orgId)?.members : undefined);

  async function sacrifice_member() {
    if (!sacrifice || !name_target) {
      flash.set_error("Pick who to spend and whose name to buy.");
      return;
    }
    const err = await client.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        ShinigamiSacrifice: {
          sacrifice: slotKeyFromString(sacrifice),
          name_target: slotKeyFromString(name_target),
        },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Trade one of your own to a shinigami for another player's true name. The sacrifice dies,
    and the world is told what they were spent on. The name goes to the org.
  </p>
  <span class="text-xs uppercase tracking-wide text-neutral-500">Spend</span>
  <PlayerSelect
    bind:value={sacrifice}
    placeholder="Member to sacrifice"
    ids={members}
  />
  <span class="text-xs uppercase tracking-wide text-neutral-500">For the name of</span>
  <PlayerSelect bind:value={name_target} placeholder="Whose name to buy" />
  <button
    class="rounded-md bg-red-600 px-3 py-2 text-sm font-medium text-white hover:bg-red-500"
    onclick={sacrifice_member}
  >
    Make the trade
  </button>
  <FlashDisplay {flash} />
</div>
