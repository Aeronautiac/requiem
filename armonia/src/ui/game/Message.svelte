<script lang="ts">
  // A single Discord-style message row: sender name, muted timestamp, wrapped
  // content. The parent resolves the sender to a display string (it owns the game
  // context and the ActorDisplay -> name logic); this component is pure presentation.
  interface Props {
    sender: string;
    content: string;
    timestamp: number;
  }
  let { sender, content, timestamp }: Props = $props();

  const time = $derived(
    new Date(timestamp).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    }),
  );
</script>

<div class="px-4 py-1 hover:bg-neutral-800/40">
  <div class="flex items-baseline gap-2">
    <span class="font-medium text-neutral-100">{sender}</span>
    <span class="text-xs text-neutral-500">{time}</span>
  </div>
  <div class="whitespace-pre-wrap break-words text-sm text-neutral-300">
    {content}
  </div>
</div>
