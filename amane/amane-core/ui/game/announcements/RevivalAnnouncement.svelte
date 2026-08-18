<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";
  import MentionText from "../MentionText.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: { target_id: string; message: string }; view: GameView; timestamp: number } =
    $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-revival)"
  description="Revival"
>
  <Name id={data.target_id} {view} chip /> never actually died!
  {#if data.message}
    <div class="mt-1 italic whitespace-pre-wrap text-neutral-300">
      <MentionText content={data.message} {view} />
    </div>
  {/if}
</Announcement>