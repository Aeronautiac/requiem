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
  const view = $derived(game.view_of(ui.viewer));

  // Leadership stays inside the org, so only its own members can be named. Whether a successor is
  // required at all depends on the org's transfer policy, which the engine enforces.
  const members = $derived(orgId ? view.orgs.get(orgId)?.members : undefined);

  let successor = $state("");
  const flash = new Flash();

  async function resign() {
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        LeaderResign: {
          successor: successor ? slotKeyFromString(successor) : null,
        },
      }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Step down as leader. Leadership passes on per the org's policy — some orgs require you to
    name your successor, others decide for themselves.
  </p>
  <PlayerSelect bind:value={successor} placeholder="Successor (if your org requires one)" ids={members} />
  <button
    class="rounded-md bg-neutral-700 px-3 py-2 text-sm font-medium text-white hover:bg-neutral-600"
    onclick={resign}
  >
    Resign leadership
  </button>
  <FlashDisplay {flash} />
</div>
