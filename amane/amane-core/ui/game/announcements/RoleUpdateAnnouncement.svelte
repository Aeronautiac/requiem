<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { Role } from "../../../bindings";
  import { roleColorVar, roleLabel } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";
  import Chip from "../Chip.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { role: Role }; view: GameView; timestamp: number } = $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-personal)"
  description="Role"
>
  Your role is now <Chip
    label={roleLabel(data.role)}
    colorVar={roleColorVar(data.role)}
  />.
</Announcement>
