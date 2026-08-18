<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import { nameLabel } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";
  import MentionText from "../MentionText.svelte";

  let { data, view, timestamp }: { data: { target_id: string; true_name: string; death_message: string }; view: GameView; timestamp: number } = $props();
</script>

<!-- Beat 1 of the staged reveal: the death and the name behind it. The role and any inheritance
     follow as their own events (DeathRole / DeathTransfer), timed apart. -->
<Announcement {view} {timestamp} color="var(--color-event-death)" description="Death">
  <Name id={data.target_id} {view} chip /> has died.
  {#if data.death_message}
    <div class="mt-1 italic whitespace-pre-wrap text-neutral-300">
      <MentionText content={data.death_message} {view} />
    </div>
  {/if}
  <div class="mt-1 text-neutral-400">
    Their true name was <span class="text-neutral-200">{nameLabel(data.true_name)}</span>.
  </div>
</Announcement>