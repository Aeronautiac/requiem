<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import MentionText from "../MentionText.svelte";

  let { data, view, timestamp }: { data: { content: string }; view: GameView; timestamp: number } = $props();
</script>

<Announcement {view} {timestamp}
  color="var(--color-event-anonymous)"
  description="Anonymous Announcement"
>
  <div class="whitespace-pre-wrap">
    <MentionText content={data.content} {view} />
  </div>
</Announcement>
