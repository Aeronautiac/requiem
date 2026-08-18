<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { TapInOutcome } from "../../../bindings";
  import { formatDuration } from "../../../lib/utils";
  import Announcement from "../Announcement.svelte";

  let {
    data,
    view,
    timestamp,
  }: {
    data: { contact_id: number; outcome: TapInOutcome };
    view: GameView;
    timestamp: number;
  } = $props();

  // A miss says WHICH miss on purpose: a contact channel is loggable unless an admin turned it off,
  // so hitting a dark one is a real finding rather than a polite way of saying the number was wrong.
  function tap_in_text(outcome: TapInOutcome): string {
    if (outcome === "NoSuchContact") {
      return `Contact ${data.contact_id} does not exist.`;
    }
    if (outcome === "NotLoggable") {
      return `Contact ${data.contact_id} exists, but is not loggable.`;
    }
    const scope =
      outcome.Found.range === null
        ? "everything ever sent there"
        : `the last ${formatDuration(outcome.Found.range)}`;
    return `Tapped into contact ${data.contact_id}. Revealing ${scope}.`;
  }
</script>

<Announcement
  {view}
  {timestamp}
  color={typeof data.outcome === "string"
    ? "var(--color-event-nothing)"
    : "var(--color-event-tap)"}
  description="Tap In"
  content={tap_in_text(data.outcome)}
/>
