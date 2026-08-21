<script lang="ts">
  // The channel-stream card for a vote that opened or closed — the counterpart to the interactive
  // PollCard in the panel, rendered with the same Announcement base as every other event so it reads
  // like the rest of the feed. It carries the subject and the proposed ability's arguments so a
  // notice says what was actually on the table rather than just "Vote started", and for a closed
  // poll it states how it ended. Read-only: voting is the panel's job.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import {
    pollSubjectArgs,
    pollSubjectHeading,
  } from "../../game/helpers.svelte";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { GameView } from "../../game/view.svelte";
  import type { PollOutcome, PollSubject } from "../../bindings";
  import Announcement from "./Announcement.svelte";

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

  // The description and accent read like every other announcement. Accept and Reject are the two
  // fixed accept/reject options, so a resolved vote reads as passed or rejected; a generic poll
  // names its winning option instead, and a poll that ended without one stays on the neutral accent.
  const status = $derived.by(() => {
    if (outcome === null)
      return { label: "Vote Started", color: "var(--color-event-vote)" };
    if (outcome === "Cancelled")
      return { label: "Vote Cancelled", color: "var(--color-event-nothing)" };
    if (outcome === "Inconclusive")
      return { label: "Vote Closed — No Decision", color: "var(--color-event-nothing)" };
    const won = view.polls.get(poll_id)?.options[outcome.Resolved];
    const wonLabel =
      won === undefined ? "" : typeof won.label === "string" ? won.label : won.label.Generic;
    if (wonLabel === "Accept")
      return { label: "Vote Passed", color: "var(--color-event-revival)" };
    if (wonLabel === "Reject")
      return { label: "Vote Rejected", color: "var(--color-event-death)" };
    return { label: wonLabel ? `Result: ${wonLabel}` : "Vote Resolved", color: "var(--color-event-vote)" };
  });
</script>

<Announcement {view} {timestamp} color={status.color} description={status.label}>
  <span>{heading}</span>
  {#if opener}
    <span class="ml-1 text-[0.7rem] text-neutral-500">by {view.actor_name(opener)}</span>
  {/if}
  {#if args.length > 0}
    <div class="mt-1 flex flex-col gap-0.5 border-l border-neutral-700 pl-2">
      {#each args as arg (arg.label)}
        <span class="text-[0.7rem] text-neutral-400">
          <span class="text-neutral-500">{arg.label}:</span>
          {arg.value}
        </span>
      {/each}
    </div>
  {/if}
</Announcement>
