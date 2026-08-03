<script lang="ts">
  // Pure presentation; the parent resolves the sender to a display string. Content still carries
  // raw mention tokens, which MentionText resolves against `players` — the one thing a mention
  // needs a view for.
  //
  // `grouped` only drops the sender header for a continuation of an uninterrupted run from the
  // same sender. Every row stays its own hover target and carries its own time.
  import { formatTime } from "../../lib/utils";
  import type { Player } from "../../game/types";
  import MentionText from "./MentionText.svelte";

  interface Props {
    sender: string;
    content: string;
    players: ReadonlyMap<string, Player>;
    timestamp: number;
    grouped?: boolean;
    // Whether this message names the viewer. Tints the whole row and keeps the tint on hover,
    // rather than the ordinary hover highlight, so a ping stands out in a busy channel.
    mentioned?: boolean;
    // Last message of its chain — the next row is a different sender, a non-message, or nothing.
    // Only the tail of a chain carries the block's bottom spacing, so a header isn't shoved away
    // from its own continuation lines.
    last?: boolean;
  }
  let {
    sender,
    content,
    players,
    timestamp,
    grouped = false,
    mentioned = false,
    last = false,
  }: Props = $props();

  const time = $derived(formatTime(timestamp));
</script>

<div
  class="px-4 pt-0.5 {grouped ? '' : 'mt-2 first:mt-0'} {last
    ? 'pb-1.5'
    : 'pb-0.5'} {mentioned
    ? 'bg-amber-400/10 shadow-[inset_2px_0_0_rgba(251,191,36,0.8)]'
    : 'hover:bg-neutral-800/40'}"
  title={grouped ? time : undefined}
>
  {#if !grouped}
    <div class="flex items-baseline gap-2">
      <span class="font-medium text-neutral-100">{sender}</span>
      <span class="text-xs text-neutral-500">{time}</span>
    </div>
  {/if}
  <div class="whitespace-pre-wrap break-words text-sm text-neutral-300">
    <MentionText {content} {players} />
  </div>
</div>
