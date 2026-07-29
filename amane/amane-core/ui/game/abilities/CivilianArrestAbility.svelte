<script lang="ts">
  import { execErrorText } from "../../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game/state.svelte";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let target = $state("");
  const flash = new Flash();

  async function arrest() {
    if (!target) {
      flash.set_error("Pick someone to arrest.");
      return;
    }
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        CivilianArrest: { target: slotKeyFromString(target) },
      }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Call a public arrest vote against a player. Any present player may vote; if a
    majority agrees, they're jailed for a while and then released.
  </p>
  <PlayerSelect bind:value={target} placeholder="Arrest target" />
  <button
    class="rounded-md bg-orange-600 px-3 py-2 text-sm font-medium text-white hover:bg-orange-500"
    onclick={arrest}
  >
    Call arrest vote
  </button>
  <FlashDisplay {flash} />
</div>
