<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { ActorDisplay } from "../../../bindings";
  import Announcement from "../Announcement.svelte";
  import ActorDisplayChip from "../ActorDisplay.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { user: ActorDisplay }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-reveal)"
  description="Shinigami Eye Deal"
>
  <ActorDisplayChip display={data.user} {view} /> has taken the shinigami eye deal.
</Announcement>
