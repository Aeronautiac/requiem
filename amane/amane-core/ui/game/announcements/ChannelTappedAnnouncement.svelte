<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";

  let {
    view,
    timestamp,
  }: { data: Record<string, never>; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-surveillance)"
  description="Tap In"
  content="This channel has been tapped into. A log of the channel up to this point has been sent to the tapper."
/>
