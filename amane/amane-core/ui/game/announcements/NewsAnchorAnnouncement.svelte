<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";
  import Chip from "../Chip.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { target_id: string | null }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-news-anchor)"
  description="News Anchor"
>
  {#if data.target_id}
    <Name id={data.target_id} {view} chip /> is now the
    <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
  {:else}
    The <Chip label="News Anchor" colorVar="var(--color-news-anchor)" /> position
    is now vacant.
  {/if}
</Announcement>
