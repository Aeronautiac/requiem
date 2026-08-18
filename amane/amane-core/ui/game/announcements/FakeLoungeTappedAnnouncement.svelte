<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { ActorDisplay } from "../../../bindings";
  import Announcement from "../Announcement.svelte";
  import ActorDisplayChip from "../ActorDisplay.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { display: ActorDisplay }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-surveillance)"
  description="Fake Lounge Read"
>
  Your fake lounge was tapped into by <ActorDisplayChip
    display={data.display}
    {view}
  />.
</Announcement>
