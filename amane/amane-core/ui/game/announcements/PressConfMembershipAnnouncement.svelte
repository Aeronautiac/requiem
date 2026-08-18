<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Chip from "../Chip.svelte";

  let { data, view, timestamp }: { data: { in_conf: boolean }; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp} color="var(--color-event-personal)" description="Press Conference">
  {#if data.in_conf}
    You joined the <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
  {:else}
    You left the <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
  {/if}
</Announcement>
