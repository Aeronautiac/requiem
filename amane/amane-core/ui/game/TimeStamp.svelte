<script lang="ts">
  // The one way a game moment is shown, shared so every row — message, announcement, contact log —
  // renders the same value the same way and right-aligned in the same position. Game time inline
  // (raw elapsed units since the game began), real-world wall time on hover.
  //
  // Rendered as a small bordered chip so the row reads it as a tappable/hoverable element rather
  // than stray text. The hover tooltip is portaled to <body> with `position: fixed` (see the
  // `tooltip` action), so it paints on top of everything and is never clipped by the chat scroller.
  import { formatTime, wallTimeOf } from "../../lib/utils";
  import { tooltip } from "../../lib/tooltip";
  import type { GameView } from "../../game/view.svelte";

  // `mono` keeps the tight scanning-row look for contact logs; messages and announcements are
  // proportional.
  let {
    timestamp,
    view,
    mono = false,
  }: { timestamp: number; view?: GameView; mono?: boolean } = $props();

  // Real wall time on hover, shown immediately. Falls back to the game time until the server's
  // clock anchor has arrived, so a row always has a meaningful tooltip.
  const tip = $derived(wallTimeOf(timestamp, view?.game_clock) ?? formatTime(timestamp));
</script>

<span
  use:tooltip
  data-tip={tip}
  class="shrink-0 rounded border border-neutral-700/80 bg-neutral-800/60 px-1.5 py-px text-xs tabular-nums text-neutral-400 {mono
    ? 'font-mono'
    : ''}"
>
  {formatTime(timestamp)}
</span>

