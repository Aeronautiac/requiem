<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString, type Role } from "../../../bindings";
  import { ROLES } from "../../../constants";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let target = $state("");
  let role = $state<Role>(ROLES[0]);
  const flash = new Flash();

  async function contact() {
    if (!target) {
      flash.set_error("Pick a target.");
      return;
    }
    const err = await game.dispatch(
      useAbilityRequest(ui.viewer, abilityId, {
        FalseAnonymousContact: { target: slotKeyFromString(target), role },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Open an anonymous lounge with a player — they won't see who you are, and the
    role you show them is one you choose to pose as.
  </p>
  <PlayerSelect bind:value={target} placeholder="Target" />
  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    Role to pose as
    <select
      bind:value={role}
      class="w-full rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
    >
      {#each ROLES as r (r)}
        <option value={r}>{r}</option>
      {/each}
    </select>
  </label>
  <button
    class="rounded-md bg-purple-500 px-3 py-2 text-sm font-medium text-white hover:bg-purple-400"
    onclick={contact}
  >
    Contact as {role}
  </button>
  <FlashDisplay {flash} />
</div>
