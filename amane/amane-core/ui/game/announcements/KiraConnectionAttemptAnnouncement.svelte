<script lang="ts">
  import { roleColorVar, roleLabel } from "../../../game/helpers.svelte";
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Chip from "../Chip.svelte";
  import Name from "../Name.svelte";

  let {
    data,
    view,
    timestamp,
  }: {
    data: { user: string; success: boolean };
    view: GameView;
    timestamp: number;
  } = $props();
</script>

<Announcement
  {view}
  {timestamp}
  color={data.success
    ? "var(--color-event-death)"
    : "var(--color-event-nothing)"}
  description="Kira Connection"
>
  <Name id={data.user} {view} chip /> has attempted to connect with
  <Chip label={roleLabel("Kira")} colorVar={roleColorVar("Kira")} />
  {data.success ? "and it was a success." : "They failed..."}
</Announcement>
