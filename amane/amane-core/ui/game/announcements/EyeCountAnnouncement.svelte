<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";

  let { data, view, timestamp }: { data: { count: number }; view: GameView; timestamp: number } = $props();
  const c = data.count;
</script>

<Announcement {view} {timestamp} color="var(--color-event-personal)" description="Shinigami Eyes">
  You have <span class="text-neutral-200">{c}</span> eye{c === 1 ? "" : "s"} remaining.
</Announcement>
