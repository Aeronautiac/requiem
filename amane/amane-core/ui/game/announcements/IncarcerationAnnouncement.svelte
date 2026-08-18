<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import { formatDuration } from "../../../lib/utils";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let { data, view, timestamp }: { data: { victim_id: string; duration: number | null }; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp} color="var(--color-event-custody)" description="Imprisonment">
  <Name id={data.victim_id} {view} chip /> has been imprisoned{#if data.duration}
    for {formatDuration(data.duration)}{/if}.
</Announcement>
