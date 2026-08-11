<script lang="ts">
  // Deliberately not an Announcement — a log is read by scanning dozens of these, so the row
  // carries no label of its own and leans on the accent colour instead. Pure presentation: the
  // parent resolves both ends to display strings.
  import type { ContactEvent } from "../../bindings";
  import type { GameView } from "../../game/view.svelte";
  import TimeStamp from "./TimeStamp.svelte";

  interface Props {
    from: string;
    to: string;
    event: ContactEvent;
    timestamp: number;
    view: GameView;
  }
  let { from, to, event, timestamp, view }: Props = $props();

  // The verb wraps around the second name so each row reads as a sentence rather than a record to
  // decode. Colour is scanning help only — the words carry it on their own.
  const PHRASES: Record<ContactEvent, { verb: string; suffix: string; color: string }> = {
    LoungeOpened: { verb: "contacted", suffix: "", color: "#2dd4bf" },
    GroupchatAdded: { verb: "added", suffix: "to groupchat", color: "#4ade80" },
    GroupchatRemoved: { verb: "removed", suffix: "from groupchat", color: "#f87171" },
  };

  const phrase = $derived(PHRASES[event]);
</script>

<div class="px-4 py-1">
  <div
    class="flex items-baseline gap-3 rounded-md border border-l-2 border-neutral-800 bg-neutral-800/30 px-3 py-2 text-sm hover:bg-neutral-800/60"
    style="border-left-color: {phrase.color}"
  >
    <TimeStamp {timestamp} {view} mono />
    <span class="min-w-0 flex-1 text-neutral-300">
      {from}
      <span style="color: {phrase.color}">{phrase.verb}</span>
      {to}{#if phrase.suffix}<span class="text-neutral-500">&nbsp;{phrase.suffix}</span>{/if}
    </span>
  </div>
</div>
