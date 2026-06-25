<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "./game_state.svelte.ts";
  import { UI_STATE_KEY } from "./ui_state.svelte.ts";
  import type { GameState } from "./game_state.svelte.ts";
  import type { UiState } from "./ui_state.svelte.ts";
  import { ROUTER_KEY } from "$lib/router";
  import type { Router } from "$lib/router";
  import type { ActionRequest } from "./bindings";
  import { slotKeyFromString } from "./bindings";
  import { Flash } from "./flash.svelte.ts";
  import FlashDisplay from "./Flash.svelte";
  import { viewerToActor } from "./types";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const router = getContext<Router>(ROUTER_KEY);

  let expanded = $state<string | null>(null);
  const flash = new Flash();

  async function contact(target_id_str: string, ability_id_str: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: Date.now(),
      payload: {
        UseAbility: {
          ability_id: slotKeyFromString(ability_id_str),
          ability_args: { Contact: { target_id: slotKeyFromString(target_id_str) } },
        },
      },
    };
    const err = game.process_response(await router.sendAction(request));
    if (err) {
      flash.set_error(`Contact failed: ${err}`);
    } else {
      flash.set_success("Contact sent.");
      expanded = null;
    }
  }

  function contact_abilities() {
    return [...(game.views.get(ui.viewer)?.abilities.entries() ?? [])]
      .filter(([, av]) => av.name === "Contact");
  }
</script>

<div class="flex flex-col gap-0.5 p-2">
  {#each game.players.entries() as [id, player] (id)}
    <div class="rounded text-sm">
      <button
        class="w-full text-left px-2 py-1 rounded text-neutral-300 hover:bg-neutral-800"
        onclick={() => { expanded = expanded === id ? null : id; flash.error = null; flash.success = null; }}
      >
        {player.display_name}
      </button>

      {#if expanded === id}
        <div class="ml-2 mt-0.5 flex flex-col gap-0.5">
          {#each contact_abilities() as [ability_id, av] (ability_id)}
            <button
              class="px-2 py-0.5 text-xs text-left rounded text-neutral-400 hover:bg-neutral-800"
              onclick={() => contact(id, ability_id)}
            >
              Contact ({av.usages_remaining}, resets in {av.iterations_to_reset})
            </button>
          {/each}
          <FlashDisplay {flash} />
        </div>
      {/if}
    </div>
  {/each}

  {#if game.players.size === 0}
    <p class="px-2 py-1 text-xs text-neutral-600">No players</p>
  {/if}

</div>
