<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

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
      return "Their notebook(s) and their transferrable abilities have";
    }
    return data.notebook_transferred
      ? "Their notebook(s) have"
      : "Their transferrable abilities have";
  }
</script>

<!-- Beat 3: what they left behind. -->
<Announcement
  {view}
  {timestamp}
  color="var(--color-event-death)"
  description="Inheritance"
>
  <Name id={data.target_id} {view} chip /> had some notable possessions.
  {death_transfer_text()} been given to the person responsible for their death.
</Announcement>
