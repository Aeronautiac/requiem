<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { iteration: number }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-alarm)"
  description={data.iteration === 1 ? "The Game Begins" : "New Day"}
>
  {#if data.iteration === 1}
    <strong>Day 1</strong>. Various interactions have now been unlocked.
  {:else}
    <strong>Day {data.iteration}</strong>. Cooldowns have been progressed, and
    statuses have been cleared.
  {/if}
</Announcement>
