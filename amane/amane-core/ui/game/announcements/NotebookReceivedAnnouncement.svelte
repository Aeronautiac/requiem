<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";

  let { data, view, timestamp }: { data: Record<string, never>; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp}
  color="var(--color-event-death)"
  description="Notebook"
  content="A notebook has come into your possession."
/>
