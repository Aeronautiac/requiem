<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import {
    actorLabel,
    awaitingHost,
    execErrorText,
    phaseLabel,
    playerLabel,
  } from "../../game/helpers.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { ProsecutionData } from "../../game/types";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { Action, ActorDisplay, ProsecutionPhaseView } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  // One prosecution, sized for the top panel's horizontal row. Everything a prosecution needs to be
  // driven (signal, advance, pick counsel, open its trial) lives here — the panel is just the strip
  // that lays these out.
  let { id, data }: { id: string; data: ProsecutionData } = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);

  const flash = new Flash();
  let lawyer_choice = $state("");

  const view = $derived(game.view_of(ui.viewer));
  const is_frozen = $derived(view.frozen(view.prosecution_viewport(id)));

  function display_string(display: ActorDisplay): string {
    return actorLabel(display, view.players);
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

  // Which side of a prosecution the viewer is on. Read from what the engine told this view
  // privately, never inferred from the displays — an anonymous prosecutor reads Mysterious in their
  // own copy of the snapshot.
  function my_side(): "prosecution" | "defense" | null {
    const side = view.own_prosecutions.get(id);
    if (side === undefined) return null;
    return side === "Prosecutor" ? "prosecution" : "defense";
  }

  // Which side holds the floor during a Presentation subphase, if any.
  function presentation_floor(phase: ProsecutionPhaseView): "prosecution" | "defense" | null {
    if (phase === "Voting" || !("Trial" in phase)) return null;
    const trial = phase.Trial;
    if ("Prosecutor" in trial) return trial.Prosecutor === "Presentation" ? "prosecution" : null;
    if ("Defense" in trial) return trial.Defense === "Presentation" ? "defense" : null;
    return null;
  }

  // Whether the viewer is defence counsel — counsel has no side of their own, but ends the defence's
  // presentation exactly as the defendant does, and no view remembers it, so it is read off the snapshot.
  function is_lawyer(): boolean {
    if (ui.viewer === "Admin") return false;
    const display = data.lawyer_display;
    return (
      display !== null &&
      typeof display !== "string" &&
      "Raw" in display &&
      slotKeyToString(display.Raw) === ui.viewer
    );
  }

  // The signal the viewer still has to give, if this phase takes one from them.
  const my_signal = $derived.by((): { verb: string } | null => {
    const side = my_side();
    const floor = presentation_floor(data.phase);
    const holdsFloor =
      floor !== null &&
      (floor === "prosecution"
        ? side === "prosecution"
        : side === "defense" || is_lawyer());

    if (holdsFloor) return is_frozen ? null : { verb: "end my turn" };

    if (side === null || awaitingHost(data.phase) || is_frozen) return null;
    const mine = signals(data.phase).find((s) => s.side === side);
    return mine && !mine.done ? { verb: mine.verb } : null;
  });

  // Counsel is the defendant's to choose, once, while they are still in custody.
  const can_pick_lawyer = $derived(
    my_side() === "defense" &&
      data.lawyer_display === null &&
      typeof data.phase !== "string" &&
      "Custody" in data.phase &&
      !is_frozen,
  );

  const lawyer_candidates = $derived(
    [...view.players.keys()].filter((pid) => pid !== ui.viewer),
  );

  function open_channel() {
    if (data.trial_channel) ui.select_channel(data.trial_channel);
  }

  async function run(payload: Action, ok: string) {
    const reply = await session.submit_action({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    });
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success(ok);
  }
  const advance = () =>
    run({ AdvanceProsecution: { prosecution_id: slotKeyFromString(id) } }, "Advanced.");
  const terminate = () =>
    run(
      { TerminateProsecution: { prosecution_id: slotKeyFromString(id), verdict: null } },
      "Terminated.",
    );
  const send_signal = () =>
    run({ SignalReady: { prosecution_id: slotKeyFromString(id) } }, "Signalled.");
  const pick_lawyer = () => {
    if (!lawyer_choice) return;
    run(
      {
        SelectLawyer: {
          prosecution_id: slotKeyFromString(id),
          lawyer_id: slotKeyFromString(lawyer_choice),
        },
      },
      "Counsel selected.",
    );
  };
</script>

<div
  class="flex w-72 shrink-0 flex-col gap-1.5 border border-neutral-700 bg-gradient-to-b from-neutral-800 to-neutral-900 p-3 shadow-md"
>
  <div class="flex items-center justify-between gap-2">
    <span class="text-sm font-medium text-neutral-100">
      {display_string(data.prosecutor_display)}
      <span class="text-neutral-600">vs</span>
      {display_string(data.defendant_display)}
    </span>
    {#if is_frozen}
      <span
        class="bg-amber-900/60 px-1.5 text-[0.6rem] uppercase tracking-wide text-amber-300"
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

  {#each signals(data.phase) as signal (signal.side)}
    <span class="text-[0.7rem] {signal.done ? 'text-emerald-500' : 'text-neutral-600'}">
      {signal.done ? "✓" : "○"}
      {signal.side}
      {signal.done ? signal.verb : `not ${signal.verb}`}
    </span>
  {/each}

  {#if can_pick_lawyer}
    <div class="flex items-center gap-1">
      <select
        class="min-w-0 flex-1 border border-neutral-800 bg-neutral-950 px-1.5 py-0.5 text-xs text-neutral-300"
        bind:value={lawyer_choice}
      >
        <option value="" disabled selected>choose counsel…</option>
        {#each lawyer_candidates as player_id (player_id)}
          <option value={player_id}>{playerLabel(player_id, view.players)}</option>
        {/each}
      </select>
      <button
        class="shrink-0 px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200 disabled:opacity-40 disabled:hover:bg-transparent"
        disabled={!lawyer_choice}
        onclick={pick_lawyer}
      >
        retain
      </button>
    </div>
  {/if}

  <div class="flex flex-wrap items-center gap-1">
    {#if data.trial_channel}
      <button
        class="px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
        onclick={open_channel}
      >
        open trial
      </button>
    {/if}

    {#if my_signal}
      <button
        class="bg-emerald-900/50 px-1.5 py-0.5 text-xs text-emerald-300 hover:bg-emerald-900"
        onclick={send_signal}
      >
        {my_signal.verb}
      </button>
    {/if}

    {#if ui.viewer === "Admin"}
      <button
        class="px-1.5 py-0.5 text-xs {awaitingHost(data.phase)
          ? 'bg-amber-900/60 text-amber-300 hover:bg-amber-900'
          : 'text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200'}"
        onclick={advance}
      >
        {awaitingHost(data.phase) ? "approve" : "advance"}
      </button>
      <button
        class="px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-red-400"
        onclick={terminate}
      >
        terminate
      </button>
    {/if}
  </div>

  <FlashDisplay {flash} />
</div>
