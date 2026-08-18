<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Chip from "../Chip.svelte";

  let { data, view, timestamp }: { data: { holding: boolean }; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp} color="var(--color-event-personal)" description="News Anchor">
  {#if data.holding}
    You are now the <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
  {:else}
    You are no longer the <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
  {/if}
</Announcement>
