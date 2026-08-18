<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { BugContext } from "../../../bindings.ts";
  import Announcement from "../Announcement.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { context: BugContext }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-surveillance)"
  description="Bugged"
  content={data.context === "Custody"
    ? "You are bugged. Your messages are being monitored while you are in custody."
    : "You have been bugged. Your messages are being monitored."}
/>
