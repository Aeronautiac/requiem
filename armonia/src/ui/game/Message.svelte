<script lang="ts">
  // A single Discord-style message row: sender name, muted timestamp, wrapped content.
  // The parent resolves the sender to a display string (it owns the game context and the
  // ActorDisplay -> name logic); this component is pure presentation.
  //
  // Every row is its own hover target (messages stay unique objects). `grouped` only drops
  // the sender header for a continuation of an uninterrupted run from the same sender within
  // a short window, tightening the run into one block; the row still carries its own time
  // (shown as a hover tooltip).
  interface Props {
    sender: string;
    content: string;
    timestamp: number;
    grouped?: boolean;
  }
  let { sender, content, timestamp, grouped = false }: Props = $props();

  const time = $derived(
    new Date(timestamp).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    }),
  );
</script>

<div
  class="px-4 hover:bg-neutral-800/40 {grouped ? 'py-0.5' : 'mt-3 pt-0.5 first:mt-0'}"
  title={grouped ? time : undefined}
>
  {#if !grouped}
    <div class="flex items-baseline gap-2">
      <span class="font-medium text-neutral-100">{sender}</span>
      <span class="text-xs text-neutral-500">{time}</span>
    </div>
  {/if}
  <div class="whitespace-pre-wrap break-words text-sm text-neutral-300">
    {content}
  </div>
</div>
