<script lang="ts">
  // The sender resolves against the view — label via actorLabel, colour via displayColorVar, so the
  // header matches how that name renders everywhere else. Content still carries raw mention tokens,
  // which MentionText resolves against the same view.
  //
  // `grouped` only drops the sender header for a continuation of an uninterrupted run from the
  // same sender. Every row stays its own hover target and carries its own time.
  import { actorLabel, displayColorVar } from "../../game/helpers.svelte";
  import type { GameView } from "../../game/view.svelte";
  import type { ActorDisplay } from "../../bindings";
  import { slotKeyToString } from "../../bindings";
  import MentionText from "./MentionText.svelte";
  import Name from "./Name.svelte";
  import TimeStamp from "./TimeStamp.svelte";

  interface Props {
    senderDisplay: ActorDisplay;
    content: string;
    view: GameView;
    timestamp: number;
    grouped?: boolean;
    // Whether this message names the viewer. Tints the whole row and keeps the tint on hover,
    // rather than the ordinary hover highlight, so a ping stands out in a busy channel.
    mentioned?: boolean;
    // Last message of its chain — the next row is a different sender, a non-message, or nothing.
    // Only the tail of a chain carries the block's bottom spacing, so a header isn't shoved away
    // from its own continuation lines.
    last?: boolean;
  }
  let {
    senderDisplay,
    content,
    view,
    timestamp,
    grouped = false,
    mentioned = false,
    last = false,
  }: Props = $props();

  const senderLabel = $derived(actorLabel(senderDisplay, view.players));
  const senderColor = $derived(displayColorVar(senderDisplay, view));
  // A player sender routes through Name so its header is clickable (opens the profile menu) and
  // coloured like that name everywhere else; roles / orgs / System / Mysterious have no player to
  // act on, so they stay plain coloured text.
  const senderPlayer = $derived(
    typeof senderDisplay !== "string" && "Raw" in senderDisplay
      ? slotKeyToString(senderDisplay.Raw)
      : null,
  );
</script>

<div
  class="px-4 pt-0.5 {grouped ? '' : 'mt-2 first:mt-0'} {last
    ? 'pb-1.5'
    : 'pb-0.5'} {mentioned
    ? 'bg-amber-400/10 shadow-[inset_2px_0_0_rgba(251,191,36,0.8)]'
    : 'hover:bg-neutral-800/40'}"
>
  {#if !grouped}
    <div class="flex items-baseline justify-between gap-2">
      <div class="flex min-w-0 items-baseline gap-2">
        {#if senderPlayer}
          <Name id={senderPlayer} {view} />
        {:else}
          <span class="font-medium" style="color: {senderColor}">{senderLabel}</span>
        {/if}
      </div>
      <TimeStamp {timestamp} {view} />
    </div>
  {/if}
  <div class="whitespace-pre-wrap break-words text-sm text-neutral-300">
    <MentionText {content} {view} />
  </div>
</div>
