<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";

  let {
    data,
    view,
    timestamp,
  }: {
    data: {
      target_id: string;
      notebook_transferred: boolean;
      ability_transferred: boolean;
    };
    view: GameView;
    timestamp: number;
  } = $props();

  function death_transfer_text(): string {
    if (data.notebook_transferred && data.ability_transferred) {
      return "their notebooks and their transferrable abilities have";
    }
    return data.notebook_transferred
      ? "their notebooks have"
      : "their their transferrable abilities have";
  }
</script>

<!-- Beat 3: what they left behind. -->
<Announcement
  {view}
  {timestamp}
  color="var(--color-event-death)"
  description="Inheritance"
>
  {death_transfer_text()} been passed to someone new.
</Announcement>
