<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState, ProsecutionData } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActorDisplay, ProsecutionPhaseView } from "../../bindings";
  import { slotKeyToString } from "../../bindings";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let open = $state(true);

  // Prosecutions are public, but each view holds its own snapshot (an absent viewer's may be
  // frozen/stale). Admin uses the System view; players use their own.
  const view = $derived(
    ui.viewer === "Admin" ? game.system_view() : game.views.get(ui.viewer),
  );
  const frozen = $derived(ui.viewer === "Admin" ? null : view?.frozen_prosecutions);
  const prosecutions = $derived([...(view?.prosecutions.entries() ?? [])]);

  function display_string(display: ActorDisplay): string {
    if (display === "Mysterious") return "???";
    if (display === "System") return "System";
    if ("Raw" in display)
      return game.players.get(slotKeyToString(display.Raw))?.display_name ?? "Unknown";
    if ("Role" in display) return display.Role;
    if ("Org" in display) return "Org";
    return "Unknown";
  }

  function phase_text(phase: ProsecutionPhaseView): string {
    if (phase === "Custody") return "In custody";
    if (phase === "Voting") return "Verdict vote";
    if (phase.Trial === "Prosecutor") return "Trial · prosecution speaking";
    if (phase.Trial === "Defense") return "Trial · defense speaking";
    return "Trial · debate";
  }

  // Open the trial channel in the main pane, if this prosecution has one yet.
  function open_channel(data: ProsecutionData) {
    if (data.trial_channel) ui.select_channel(data.trial_channel);
  }
</script>

<div class="flex flex-col gap-1 border-b border-neutral-800 p-2">
  <button
    class="flex items-center gap-1 px-2 py-1 text-xs font-medium uppercase tracking-wide text-neutral-400 hover:text-neutral-200"
    onclick={() => (open = !open)}
  >
    <span class="text-[0.6rem]">{open ? "▾" : "▸"}</span>
    Prosecutions
    {#if prosecutions.length > 0}
      <span class="ml-1 rounded bg-neutral-800 px-1.5 text-[0.65rem] text-neutral-400">
        {prosecutions.length}
      </span>
    {/if}
  </button>

  {#if open}
    {#if prosecutions.length === 0}
      <p class="px-2 py-1 text-xs text-neutral-600">No active prosecutions</p>
    {:else}
      {#each prosecutions as [id, data] (id)}
        <div class="flex flex-col gap-1.5 rounded border border-neutral-800 px-2 py-2">
          <div class="flex items-center justify-between gap-2">
            <span class="text-sm text-neutral-200">
              {display_string(data.prosecutor_display)}
              <span class="text-neutral-600">vs</span>
              {display_string(data.defendant_display)}
            </span>
            {#if frozen?.has(id)}
              <span
                class="rounded bg-amber-900/60 px-1.5 text-[0.6rem] uppercase tracking-wide text-amber-300"
                title="You lost presence — showing the last state you received."
              >
                frozen
              </span>
            {/if}
          </div>

          <span class="text-[0.7rem] text-neutral-500">{phase_text(data.phase)}</span>

          {#if data.trial_channel}
            <button
              class="self-start rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
              onclick={() => open_channel(data)}
            >
              open trial
            </button>
          {/if}
        </div>
      {/each}
    {/if}
  {/if}
</div>
