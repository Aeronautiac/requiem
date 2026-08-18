<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let {
    data,
    view,
    timestamp,
  }: {
    data: { victim: string | null; kidnapper: string | null };
    view: GameView;
    timestamp: number;
  } = $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-alarm)"
  description="Kidnap Reveal"
>
  Authorities have recovered {#if data.victim}<Name
      id={data.victim}
      {view}
      chip
    />{:else}the victim{/if}. When the victim was questioned,
  {#if data.kidnapper}
    they revealed <Name id={data.kidnapper} {view} chip /> as their kidnapper.
  {:else}
    they could not identify their kidnapper(s).
  {/if}
</Announcement>
