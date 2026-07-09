<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import type { Role } from "../../../bindings";
  import { slotKeyFromString } from "../../../bindings";
  import { ROLES } from "../../../constants";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let target = $state("");
  let true_name = $state("");
  let death_message = $state("");
  let role = $state<Role>(ROLES[0]);
  let notebook_transferred = $state(false);
  let ability_transferred = $state(false);
  const flash = new Flash();

  async function fake_death() {
    if (!target) {
      flash.set_error("Pick whose death to fake.");
      return;
    }
    if (!true_name.trim()) {
      flash.set_error("A true name is required.");
      return;
    }
    const err = await game.dispatch(
      useAbilityRequest(ui.viewer, abilityId, {
        Pseudocide: {
          target_id: slotKeyFromString(target),
          true_name,
          death_message,
          role,
          notebook_transferred,
          ability_transferred,
        },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Fake a target's death (yourself included). They're revived after a delay; the
    fields below are announced as the death reveal.
  </p>

  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    Target
    <PlayerSelect bind:value={target} placeholder="Whose death to fake" />
  </label>

  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    True name revealed
    <input
      bind:value={true_name}
      placeholder="True name"
      class="w-full rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
    />
  </label>

  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    Death message
    <input
      bind:value={death_message}
      placeholder="Announced on death"
      class="w-full rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
    />
  </label>

  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    Role revealed
    <select
      bind:value={role}
      class="w-full rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
    >
      {#each ROLES as r (r)}
        <option value={r}>{r}</option>
      {/each}
    </select>
  </label>

  <label class="flex items-center gap-2 text-sm text-neutral-300">
    <input type="checkbox" bind:checked={notebook_transferred} />
    Notebook transferred
  </label>
  <label class="flex items-center gap-2 text-sm text-neutral-300">
    <input type="checkbox" bind:checked={ability_transferred} />
    Abilities transferred
  </label>

  <button
    class="rounded-md bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-900 hover:bg-white"
    onclick={fake_death}
  >
    Fake death
  </button>
  <FlashDisplay {flash} />
</div>
