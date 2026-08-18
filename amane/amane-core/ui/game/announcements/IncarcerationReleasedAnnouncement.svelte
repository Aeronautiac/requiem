<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let { data, view, timestamp }: { data: { victim: string | null }; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp} color="var(--color-event-custody)" description="Release">
  {#if data.victim}<Name id={data.victim} {view} chip />{:else}A prisoner{/if} has been released.
</Announcement>
