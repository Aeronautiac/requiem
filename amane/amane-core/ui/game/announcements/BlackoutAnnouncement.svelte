<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { active: boolean }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-blackout)"
  description={data.active ? "Blackout" : "Blackout Over"}
  content={data.active ? "The world's gone dark..." : "The light returns..."}
/>
