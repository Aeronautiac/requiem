<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";
  import Chip from "../Chip.svelte";

  let { data, view, timestamp }: { data: { target_id: string; has_access: boolean }; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp}
  color="var(--color-press-conference)"
  description="Press Conference"
>
  <Name id={data.target_id} {view} chip />
  {data.has_access ? "joined the" : "left the"}
  <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
</Announcement>
