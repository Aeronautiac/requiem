<script lang="ts">
  // The one way a game moment is shown, shared so every row — message, announcement, contact log —
  // renders the same value the same way and right-aligned in the same position: raw elapsed game time
  // since the game began. There is intentionally no real-world wall time: with time travel, game time
  // is untethered from the wall clock, so a wall date has no meaning to attach to a moment.
  import { formatTime } from "../../lib/utils";
  import type { GameView } from "../../game/view.svelte";

  // `mono` keeps the tight scanning-row look for contact logs; messages and announcements are
  // proportional.
  let {
    timestamp,
    view,
    mono = false,
  }: { timestamp: number; view?: GameView; mono?: boolean } = $props();
</script>

<span
  class="shrink-0 rounded border border-neutral-700/80 bg-neutral-800/60 px-1.5 py-px text-xs tabular-nums text-neutral-400 {mono
    ? 'font-mono'
    : ''}"
>
  {formatTime(timestamp)}
</span>

