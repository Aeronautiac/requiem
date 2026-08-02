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
  import type { Action, ActionRequest, ActorDisplay, ProsecutionPhaseView } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);

  const flash = new Flash();
  let open = $state(true);

  // Who the defendant has picked out of the list, per prosecution, before they commit to it.
  let lawyer_choice = $state<Record<string, string>>({});

  // Prosecutions are public, but each view holds its own snapshot and an absent viewer's may be
  // frozen. System enters no viewport, so nothing it holds is ever frozen — no check needed here
  // to say so.
  const view = $derived(game.view_of(ui.viewer));
  const is_frozen = (id: string) => view.frozen(view.prosecution_viewport(id));
  const prosecutions = $derived([...view.prosecutions.entries()]);

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
  // privately, never inferred from the displays — an anonymous prosecutor reads Mysterious in
  // their own copy of the snapshot, so the displays cannot answer this for the one player who
  // most needs it answered.
  //
  // Named in the vocabulary the signal flags use, so the two line up without a second mapping.
  function my_side(id: string): "prosecution" | "defense" | null {
    const side = view.own_prosecutions.get(id);
    if (side === undefined) return null;
    return side === "Prosecutor" ? "prosecution" : "defense";
  }

  // Which side holds the floor during a Presentation subphase, if any. The floor holder ends the
  // slot themselves instead of waiting out the clock; no side holds a floor in any other phase.
  function presentation_floor(phase: ProsecutionPhaseView): "prosecution" | "defense" | null {
    if (phase === "Voting" || !("Trial" in phase)) return null;
    const trial = phase.Trial;
    if ("Prosecutor" in trial) {
      return trial.Prosecutor === "Presentation" ? "prosecution" : null;
    }
    if ("Defense" in trial) {
      return trial.Defense === "Presentation" ? "defense" : null;
    }
    return null;
  }

  // Whether the viewer is defence counsel. Counsel has no side of their own — the engine tells only
  // Prosecution/Defendant — but they end the defence's presentation exactly as the defendant does,
  // and no view remembers it, so it must be read off the snapshot.
  //
  // Matched on the raw slot only: the display is raw whenever it can name a player, and counsel is
  // never disguised.
  function is_lawyer(id: string, data: ProsecutionData): boolean {
    if (ui.viewer === "Admin") return false;
    const display = data.lawyer_display;
    return (
      display !== null &&
      typeof display !== "string" &&
      "Raw" in display &&
      slotKeyToString(display.Raw) === ui.viewer
    );
  }

  // The signal the viewer still has to give, if this phase takes one from them. A phase parked on
  // a host sits at a boundary only in the held phases, and a frozen snapshot is not something to
  // act on — so neither the ready/done signal nor the floor hold offers itself there.
  function my_signal(id: string, data: ProsecutionData): { verb: string } | null {
    const side = my_side(id);
    const floor = presentation_floor(data.phase);
    const holdsFloor =
      floor !== null &&
      (floor === "prosecution"
        ? side === "prosecution"
        : side === "defense" || is_lawyer(id, data));

    if (holdsFloor) {
      // Single-sided: no flag to pair, the slot ends at once (SignalReady, System-held advance).
      return is_frozen(id) ? null : { verb: "end my turn" };
    }

    if (side === null || awaitingHost(data.phase) || is_frozen(id)) return null;
    const mine = signals(data.phase).find((s) => s.side === side);
    return mine && !mine.done ? { verb: mine.verb } : null;
  }

  // Counsel is the defendant's to choose, once, while they are still in custody.
  const can_pick_lawyer = (id: string, data: ProsecutionData) =>
    my_side(id) === "defense" &&
    data.lawyer_display === null &&
    typeof data.phase !== "string" &&
    "Custody" in data.phase &&
    !is_frozen(id);

  const lawyer_candidates = $derived(
    [...view.players.keys()].filter((id) => id !== ui.viewer),
  );

  function open_channel(data: ProsecutionData) {
    if (data.trial_channel) ui.select_channel(data.trial_channel);
  }

  // Advancing works from any phase at any moment — it IS the decision, not a confirmation of one
  // the engine already made. A non-autonomous prosecution parked at a boundary needs this to move.
  async function run(payload: Action, ok: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    };
    const reply = await session.submit_action(request);
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success(ok);
  }
  const advance = (id: string) =>
    run({ AdvanceProsecution: { prosecution_id: slotKeyFromString(id) } }, "Advanced.");
  const terminate = (id: string) =>
    run(
      { TerminateProsecution: { prosecution_id: slotKeyFromString(id), verdict: null } },
      "Terminated.",
    );
  const send_signal = (id: string) =>
    run({ SignalReady: { prosecution_id: slotKeyFromString(id) } }, "Signalled.");
  const pick_lawyer = (id: string) => {
    const lawyer = lawyer_choice[id];
    if (!lawyer) return;
    run(
      {
        SelectLawyer: {
          prosecution_id: slotKeyFromString(id),
          lawyer_id: slotKeyFromString(lawyer),
        },
      },
      "Counsel selected.",
    );
  };
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

          <!-- Counsel is chosen once, and only while the defendant is still in custody. -->
          {#if can_pick_lawyer(id, data)}
            <div class="flex items-center gap-1">
              <select
                class="min-w-0 flex-1 rounded border border-neutral-800 bg-neutral-900 px-1.5 py-0.5 text-xs text-neutral-300"
                bind:value={lawyer_choice[id]}
              >
                <option value="" disabled selected>choose counsel…</option>
                {#each lawyer_candidates as player_id (player_id)}
                  <option value={player_id}>{playerLabel(player_id, view.players)}</option>
                {/each}
              </select>
              <button
                class="shrink-0 rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200 disabled:opacity-40 disabled:hover:bg-transparent"
                disabled={!lawyer_choice[id]}
                onclick={() => pick_lawyer(id)}
              >
                retain
              </button>
            </div>
          {/if}

          <div class="flex flex-wrap items-center gap-1">
            {#if data.trial_channel}
              <button
                class="rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
                onclick={() => open_channel(data)}
              >
                open trial
              </button>
            {/if}

            <!-- The viewer's own signal. Only their side's, and only while it is still theirs to
                 give — the flags above already say where both sides stand. -->
            {#if my_signal(id, data)}
              {@const mine = my_signal(id, data)}
              <button
                class="rounded bg-emerald-900/50 px-1.5 py-0.5 text-xs text-emerald-300 hover:bg-emerald-900"
                onclick={() => send_signal(id)}
              >
                {mine?.verb}
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

    <div class="px-2 pt-1">
      <FlashDisplay {flash} />
    </div>
  {/if}
</div>
