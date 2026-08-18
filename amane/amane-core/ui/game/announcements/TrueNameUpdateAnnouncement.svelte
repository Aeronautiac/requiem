<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import { nameLabel } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { true_name: string }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-personal)"
  description="True Name"
>
  Your true name is now <strong>{nameLabel(data.true_name)}</strong>.
</Announcement>
