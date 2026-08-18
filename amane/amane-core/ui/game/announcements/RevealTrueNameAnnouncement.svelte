<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import { nameLabel } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let {
    data,
    view,
    timestamp,
  }: {
    data: { target_id: string; true_name: string };
    view: GameView;
    timestamp: number;
  } = $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-reveal)"
  description="Name Reveal"
>
  <Name id={data.target_id} {view} chip />'s true name is
  <span class="text-neutral-200">{nameLabel(data.true_name)}</span>.
</Announcement>
