<script lang="ts">
  // Pure presentation; the parent resolves the sender to a display string.
  //
  // `grouped` only drops the sender header for a continuation of an uninterrupted run from the
  // same sender. Every row stays its own hover target and carries its own time.
  import { formatTime } from "../../lib/utils";

  interface Props {
    sender: string;
    content: string;
    timestamp: number;
    grouped?: boolean;
  }
  let { sender, content, timestamp, grouped = false }: Props = $props();

  const time = $derived(formatTime(timestamp));
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
