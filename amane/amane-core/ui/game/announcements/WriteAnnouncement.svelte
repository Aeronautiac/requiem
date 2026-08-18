<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { WriteEvent } from "../../../game/types";
  import { nameLabel } from "../../../game/helpers.svelte";
  import { formatDuration } from "../../../lib/utils";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let {
    data,
    view,
    timestamp,
  }: { data: WriteEvent; view: GameView; timestamp: number } = $props();

  // success = the name matched a real player; target_saved = the kill didn't land. The writer's
  // name renders as a coloured Name in the markup; this carries only the outcome lines below it.
  function write_outcome_text(w: WriteEvent): string {
    const lines: string[] = [];
    if (!w.success) {
      lines.push("Outcome: Failure. There is nobody with that true name.");
    } else if (w.target_saved) {
      lines.push("Outcome: Something saved them...");
    } else if (w.delay > 0) {
      lines.push(
        `Outcome: Success. The target will die in ${formatDuration(w.delay)}.`,
      );
    } else {
      lines.push("Outcome: Success. The target dies immediately.");
    }
    if (w.message) lines.push(`Cause of death: ${w.message}`);
    lines.push(
      `Successes left: ${w.successes_remaining} · Attempts left: ${w.attempts_remaining}`,
    );
    return lines.join("\n");
  }

  // Red = lethal, amber = valid-but-saved, grey = no match.
  function write_event_color(w: WriteEvent): string {
    if (!w.success) return "var(--color-event-nothing)";
    if (w.target_saved) return "var(--color-event-alarm)";
    return "var(--color-event-death)";
  }
</script>

<Announcement
  {view}
  {timestamp}
  color={write_event_color(data)}
  description="Notebook Write"
>
  <Name id={data.user_id} {view} chip /> wrote the name
  <span class="text-neutral-200">"{nameLabel(data.true_name)}"</span>.
  <div class="mt-1 whitespace-pre-wrap">{write_outcome_text(data)}</div>
</Announcement>

