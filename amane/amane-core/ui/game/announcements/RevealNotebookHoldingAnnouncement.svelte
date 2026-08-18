<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let { data, view, timestamp }: { data: { target_id: string; holding: boolean }; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp} color="var(--color-event-reveal)" description="Notebook Check">
  <Name id={data.target_id} {view} chip /> is {data.holding ? "" : "not "}currently
  holding a notebook.
</Announcement>
