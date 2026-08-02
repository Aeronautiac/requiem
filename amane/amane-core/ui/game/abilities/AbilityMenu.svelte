<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game/state.svelte";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game/state.svelte";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import type { AbilityName, OrgAbility } from "../../../bindings";
  import Dialog from "../../kit/Dialog.svelte";
  import Button from "../../kit/Button.svelte";
  import AbilityCard from "./AbilityCard.svelte";
  import {
    ABILITY_UIS,
    EXCLUDED_ABILITIES,
    prettyAbility,
  } from "./registry";

  // When `orgId` is set this menu drives that org's abilities (dispatching UseOrgAbility);
  // otherwise it drives the current viewer's personal abilities. Same UI either way, so
  // an org menu looks identical to the personal one — it's just a separate instance.
  let { orgId }: { orgId?: string } = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  let open = $state(false);
  let selectedId = $state<string | null>(null); // ability instance being configured

  // The abilities this menu lists: an org's shared set, or the viewer's own.
  const source = $derived(
    orgId ? view.orgs.get(orgId)?.abilities : view.abilities,
  );

  // The abilities, minus ones surfaced through a dedicated widget.
  const listed = $derived.by(() => {
    const out: {
      id: string;
      name: AbilityName;
      successUsages: number;
      failureUsages: number;
      resets: number;
      requirements?: OrgAbility;
    }[] = [];
    for (const [id, av] of source ?? []) {
      if (EXCLUDED_ABILITIES.has(av.name)) continue;
      out.push({
        id,
        name: av.name,
        successUsages: av.success_usages_remaining,
        failureUsages: av.failure_usages_remaining,
        resets: av.iterations_to_reset,
        requirements: av.requirements,
      });
    }
    return out;
  });

  const selectedAbility = $derived(
    selectedId ? source?.get(selectedId) : undefined,
  );
  const SelectedUi = $derived(
    selectedAbility ? ABILITY_UIS[selectedAbility.name] : undefined,
  );

  function close() {
    open = false;
    selectedId = null;
  }
</script>

{#snippet drilledTitle()}
  <span class="flex items-center gap-2">
    <button
      class="text-ink-dim hover:text-ink"
      onclick={() => (selectedId = null)}
      aria-label="Back to abilities"
    >
      ←
    </button>
    {prettyAbility(selectedAbility!.name)}
  </span>
{/snippet}

<Button variant="ghost" size="sm" onclick={() => (open = true)}>
  {orgId ? "Org abilities" : "Abilities"}
</Button>

<Dialog
  bind:open
  onOpenChange={(o) => !o && (selectedId = null)}
  class="max-w-sm"
  title={selectedId && SelectedUi && selectedAbility
    ? undefined
    : orgId
      ? "Org abilities"
      : "Abilities"}
  header={selectedId && SelectedUi && selectedAbility ? drilledTitle : undefined}
>
  {#if selectedId && SelectedUi && selectedAbility}
    <SelectedUi abilityId={selectedId} {orgId} onDone={close} />
  {:else}
    <div class="flex flex-col gap-2">
      {#each listed as ab (ab.id)}
        <AbilityCard
          name={ab.name}
          successUsages={ab.successUsages}
          failureUsages={ab.failureUsages}
          resets={ab.resets}
          hasUi={ABILITY_UIS[ab.name] != null}
          requirements={ab.requirements}
          onUse={() => (selectedId = ab.id)}
        />
      {/each}
      {#if listed.length === 0}
        <p class="py-2 text-sm text-ink-dim">No abilities.</p>
      {/if}
    </div>
  {/if}
</Dialog>
