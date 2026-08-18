<script lang="ts">
  // The one live game-time anchor: a running clock showing the current game time in the same units
  // as every timestamp (see formatTime). Without it there is no visible way to know how far into the
  // game you are -- a game moment is raw elapsed sandbox time, not a point on the wall clock. There
  // is deliberately no real-world wall time here: with time travel, game time is untethered from the
  // wall clock, so the counterpart date has no meaning.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import { formatTime } from "../../lib/utils";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The view this session is currently rendering. A derived, so a resync after time travel swaps it
  // (System is rebuilt fresh) and the clock keeps reading the live one.
  const view = $derived(game.view_of(ui.viewer));

  // game_time_now() reads Date.now() and the view's live {game_clock, sent_at} anchor at call time,
  // neither of which a $derived reliably retriggers -- a forward time skip replaces the anchor, and a
  // backward one rebuilds the whole view. So rather than trust Svelte's tracking, poll the current
  // value into a plain $state on an interval: every tick re-reads both, so the clock tracks a forward
  // jump immediately and can never freeze on a rebuilt view.
  let now = $state(0);
  $effect(() => {
    const id = setInterval(() => {
      now = view.game_time_now();
    }, 250);
    return () => clearInterval(id);
  });

  // Running the clock. Stops counting the moment the view stops telling time (no anchor yet / the
  // world rebuilt it) so an idle read shows the last known moment rather than a frozen "0".
  const running = $derived(game.view_of(ui.viewer).game_clock !== null);
</script>

<span class="flex items-center gap-1.5 text-sm tabular-nums text-neutral-500">
  <span class="uppercase tracking-wide text-neutral-600">clock</span>
  <span
    class="flex h-8 items-center rounded-md border border-edge bg-panel px-3 font-mono text-sm text-neutral-300"
  >
    {formatTime(running ? now : 0)}
  </span>
</span>