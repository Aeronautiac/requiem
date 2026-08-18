<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import { roleColorVar, roleLabel } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";
  import Chip from "../Chip.svelte";

  let { data, view, timestamp }: { data: { target_id: string; role: string }; view: GameView; timestamp: number } = $props();
</script>

<!-- Beat 2: who they turned out to be, in that role's own colour. -->
<Announcement {view} {timestamp} color={roleColorVar(data.role)} description="Role Revealed">
  <Name id={data.target_id} {view} chip /> was
  <Chip label={roleLabel(data.role)} colorVar={roleColorVar(data.role)} />.
</Announcement>
