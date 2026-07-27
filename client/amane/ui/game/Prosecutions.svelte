<script lang="ts">
  import { getContext } from "svelte";
  import {
    actorLabel,
    awaitingHost,
    GAME_STATE_KEY,
    phaseLabel,
  } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState, ProsecutionData } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { Action, ActionRequest, ActorDisplay, ProsecutionPhaseView } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const client = getContext<ClientState>(CLIENT_KEY);

  let open = $state(true);

  // Prosecutions are public, but each view holds its own snapshot (an absent viewer's may be
  // frozen/stale). Admin uses the System view; players use their own.
  const view = $derived(
    ui.viewer === "Admin" ? game.system_view() : game.views.get(ui.viewer),
  );
  // Admin's mirror is never presence-gated, so it is never stale.
  const is_frozen = (id: string) =>
    ui.viewer !== "Admin" && (view?.prosecution_frozen(id) ?? false);
  const prosecutions = $derived([...(view?.prosecutions.entries() ?? [])]);

  function display_string(display: ActorDisplay): string {
    return actorLabel(display, game.players);
  }

  const phase_text = phaseLabel;

  // The per-side signal state, for the phases that have one. Custody asks for "ready", debate for
  // "done"; presentation subphases have no signal at all.
  function signals(
    phase: ProsecutionPhaseView,
  ): { side: string; done: boolean; verb: string }[] {
    if (phase === "Voting") return [];
    if ("Custody" in phase) {
      return [
        { side: "prosecution", done: phase.Custody.prosecutor_ready, verb: "ready" },
        { side: "defense", done: phase.Custody.defense_ready, verb: "ready" },
      ];
    }
    if ("Debate" in phase.Trial) {
      return [
        { side: "prosecution", done: phase.Trial.Debate.prosecutor_done, verb: "done" },
        { side: "defense", done: phase.Trial.Debate.defense_done, verb: "done" },
      ];
    }
    return [];
  }

  // Open the trial channel in the main pane, if this prosecution has one yet.
  function open_channel(data: ProsecutionData) {
    if (data.trial_channel) ui.select_channel(data.trial_channel);
  }

  // Host controls. Advancing works from any phase and at any moment — it IS the decision, not a
  // confirmation of one the engine already made. A non-autonomous prosecution parked at a boundary
  // (awaiting_host) will not move without this.
  function run(payload: Action) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    };
    void client.dispatch(request);
  }
  const advance = (id: string) =>
    run({ AdvanceProsecution: { prosecution_id: slotKeyFromString(id) } });
  const terminate = (id: string) =>
    run({ TerminateProsecution: { prosecution_id: slotKeyFromString(id), verdict: null } });
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
            {#if is_frozen(id)}
              <span
                class="rounded bg-amber-900/60 px-1.5 text-[0.6rem] uppercase tracking-wide text-amber-300"
                title="You lost presence — showing the last state you received."
              >
                frozen
              </span>
            {/if}
          </div>

          <span class="text-[0.7rem] text-neutral-500">{phase_text(data.phase)}</span>

          {#if data.lawyer_display}
            <span class="text-[0.7rem] text-neutral-500">
              defended by {display_string(data.lawyer_display)}
            </span>
          {/if}

          <!-- Whether each side has signalled. This is the only feedback SignalReady has: without
               it a player cannot tell whether they are waiting on the other side or on the clock. -->
          {#each signals(data.phase) as signal (signal.side)}
            <span class="text-[0.7rem] {signal.done ? 'text-emerald-500' : 'text-neutral-600'}">
              {signal.done ? "✓" : "○"}
              {signal.side}
              {signal.done ? signal.verb : `not ${signal.verb}`}
            </span>
          {/each}

          <div class="flex flex-wrap items-center gap-1">
            {#if data.trial_channel}
              <button
                class="rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
                onclick={() => open_channel(data)}
              >
                open trial
              </button>
            {/if}

            {#if ui.viewer === "Admin"}
              <button
                class="rounded px-1.5 py-0.5 text-xs {awaitingHost(data.phase)
                  ? 'bg-amber-900/60 text-amber-300 hover:bg-amber-900'
                  : 'text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200'}"
                onclick={() => advance(id)}
              >
                {awaitingHost(data.phase) ? "approve" : "advance"}
              </button>
              <button
                class="rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-red-400"
                onclick={() => terminate(id)}
              >
                terminate
              </button>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  {/if}
</div>
