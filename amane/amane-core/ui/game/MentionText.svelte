<script lang="ts">
  // Renders message text with its embedded mentions turned into coloured chips. The `@` is dropped;
  // an id becomes a real name, resolved against the view being rendered. A player mention is that
  // player's name, so it renders through the shared Name component (chip form) and picks up the same
  // status colour a name has everywhere else; every other kind keeps its fixed entity accent.
  // Whitespace matters here — the parent sets `whitespace-pre-wrap`, so the {#each} body is kept
  // tight to avoid leaking stray spaces between segments.
  import {
    mentionColorVar,
    mentionLabel,
    parseMentions,
  } from "../../game/helpers.svelte";
  import type { GameView } from "../../game/view.svelte";
  import Chip from "./Chip.svelte";
  import Name from "./Name.svelte";

  interface Props {
    content: string;
    view: GameView;
  }
  let { content, view }: Props = $props();

  const segments = $derived(parseMentions(content));
</script>

{#each segments as seg}{#if "text" in seg}{seg.text}{:else if seg.mention.kind === "player"}<Name
      id={seg.mention.id}
      {view}
      chip
    />{:else}<Chip
      label={mentionLabel(seg.mention, view.players)}
      colorVar={mentionColorVar(seg.mention)}
    />{/if}{/each}
