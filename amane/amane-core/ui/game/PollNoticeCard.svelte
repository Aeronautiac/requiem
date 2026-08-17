<script lang="ts">
  // The channel-stream card for a vote that opened or closed — the counterpart to the interactive
  // PollCard in the panel. It carries the subject and the proposed ability's arguments, so a notice
  // says what was actually on the table rather than just "Vote started". Read-only: voting is the
  // panel's job. Shown for a resolved poll, and for an open one this view can see but not vote in.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import {
    pollSubjectArgs,
    pollSubjectHeading,
  } from "../../game/helpers.svelte";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { PollOutcome, PollSubject } from "../../bindings";
  import TimeStamp from "./TimeStamp.svelte";

  let {
    poll_id,
    subject,
    outcome,
    opener,
    timestamp,
  }: {
    poll_id: string;
    subject: PollSubject;
    outcome: PollOutcome | null;
    opener: string | null;
    timestamp: number;
  } = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const view = $derived(game.view_of(ui.viewer));

  const heading = $derived(pollSubjectHeading(subject, view.players));
  const args = $derived(pollSubjectArgs(subject, view.players));

  // The banner and the accent that tints the whole card. Accept and Reject are the two fixed
  // accept/reject options, so a resolved vote reads as passed or rejected; a generic poll names its
  // winning option instead, and a poll that ended without one stays on the neutral accent.
  const status = $derived.by(() => {
    if (outcome === null)
      return { label: "Vote Started", color: "var(--color-event-vote)" };
    if (outcome === "Cancelled")
      return { label: "Vote Cancelled", color: "var(--color-event-nothing)" };
    if (outcome === "Inconclusive")
      return { label: "Vote Closed — No Decision", color: "var(--color-event-nothing)" };
    const won = view.polls.get(poll_id)?.options[outcome.Resolved]?.label;
    const wonText = won === undefined ? "" : typeof won === "string" ? won : won.Generic;
    if (wonText === "Accept")
      return { label: "Vote Passed", color: "var(--color-event-revival)" };
    if (wonText === "Reject")
      return { label: "Vote Rejected", color: "var(--color-event-death)" };
    return { label: `Result: ${wonText || "resolved"}`, color: "var(--color-event-vote)" };
  });
</script>

<div class="px-3 py-0.5">
  <div
    class="relative overflow-hidden border border-l-2 border-neutral-700 px-3 py-2"
    style="border-left-color: {status.color}"
  >
    <div
      class="pointer-events-none absolute inset-0"
      style="background-color: {status.color}; opacity: 0.1"
    ></div>

    <div class="relative flex flex-col gap-1.5">
      <div class="flex items-baseline justify-between gap-2">
        <span class="flex shrink-0 items-baseline gap-2">
          <span
            class="text-[0.8rem] font-semibold uppercase tracking-wide"
            style="color: {status.color}"
          >
            {status.label}
          </span>
          {#if opener}
            <span class="text-[0.65rem] text-neutral-500">
              by {view.actor_name(opener)}
            </span>
          {/if}
        </span>
        <TimeStamp {timestamp} {view} />
      </div>

      <span class="text-sm text-neutral-200">{heading}</span>

      {#if args.length > 0}
        <div class="flex flex-col gap-0.5 border-l border-neutral-700 pl-2">
          {#each args as arg (arg.label)}
            <span class="text-[0.7rem] text-neutral-400">
              <span class="text-neutral-500">{arg.label}:</span>
              {arg.value}
            </span>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
