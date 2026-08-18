<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { target_id: string }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-alarm)"
  description="Kidnapping"
>
  <Name id={data.target_id} {view} chip /> has been kidnapped. Authorities have begun
  rescue operations, but it may be a while before they are found.
</Announcement>
