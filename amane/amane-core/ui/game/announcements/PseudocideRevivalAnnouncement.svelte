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
  color="var(--color-event-revival)"
  description="Pseudocide"
>
  It appears that <Name id={data.target_id} {view} chip /> never actually died! They
  were found half-naked and unconscious in the middle of a crowded street in Tokyo.
  When questioned, they claimed to have no recollection of how they arrived there.
  Their death appears to have been staged using a nearly perfect replica dummy. Authorities
  have now allowed them to re-enter society.
</Announcement>
