<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import type { AbilityName } from "../../../bindings";
  import * as Dialog from "$lib/components/ui/dialog";
  import AbilityCard from "./AbilityCard.svelte";
  import {
    ABILITY_UIS,
    EXCLUDED_ABILITIES,
    prettyAbility,
  } from "./registry";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let open = $state(false);
  let selectedId = $state<string | null>(null); // ability instance being configured

  // The viewer's abilities, minus ones surfaced through a dedicated widget.
  const listed = $derived.by(() => {
    const out: { id: string; name: AbilityName; usages: number; resets: number }[] =
      [];
    for (const [id, av] of game.views.get(ui.viewer)?.abilities ?? []) {
      if (EXCLUDED_ABILITIES.has(av.name)) continue;
      out.push({
        id,
        name: av.name,
        usages: av.success_usages_remaining,
        resets: av.iterations_to_reset,
      });
    }
    return out;
  });

  const selectedAbility = $derived(
    selectedId
      ? game.views.get(ui.viewer)?.abilities.get(selectedId)
      : undefined,
  );
  const SelectedUi = $derived(
    selectedAbility ? ABILITY_UIS[selectedAbility.name] : undefined,
  );

  function close() {
    open = false;
    selectedId = null;
  }
</script>

<Dialog.Root bind:open onOpenChange={(o) => !o && (selectedId = null)}>
  <Dialog.Trigger
    class="h-8 rounded-md border border-neutral-700 bg-neutral-900 px-3 text-sm text-neutral-200 hover:bg-neutral-800"
  >
    Abilities
  </Dialog.Trigger>
  <Dialog.Content class="max-w-sm">
    {#if selectedId && SelectedUi && selectedAbility}
      <Dialog.Header>
        <Dialog.Title class="flex items-center gap-2">
          <button
            class="text-neutral-500 hover:text-neutral-200"
            onclick={() => (selectedId = null)}
            aria-label="Back to abilities"
          >
            ←
          </button>
          {prettyAbility(selectedAbility.name)}
        </Dialog.Title>
      </Dialog.Header>
      <SelectedUi abilityId={selectedId} onDone={close} />
    {:else}
      <Dialog.Header>
        <Dialog.Title>Abilities</Dialog.Title>
      </Dialog.Header>
      <div class="flex flex-col gap-2">
        {#each listed as ab (ab.id)}
          <AbilityCard
            name={ab.name}
            usages={ab.usages}
            resets={ab.resets}
            hasUi={ABILITY_UIS[ab.name] != null}
            onUse={() => (selectedId = ab.id)}
          />
        {/each}
        {#if listed.length === 0}
          <p class="py-2 text-sm text-neutral-600">No abilities.</p>
        {/if}
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
