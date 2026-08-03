<script lang="ts">
  // Renders message text with its embedded mentions turned into coloured chips. The `@` is dropped;
  // an id becomes a real name, resolved against the view being rendered. Whitespace matters here —
  // the parent sets `whitespace-pre-wrap`, so the {#each} body is kept tight to avoid leaking stray
  // spaces between segments.
  import {
    mentionColorVar,
    mentionLabel,
    parseMentions,
  } from "../../game/helpers.svelte";
  import type { Player } from "../../game/types";
  import Chip from "./Chip.svelte";

  interface Props {
    content: string;
    players: ReadonlyMap<string, Player>;
  }
  let { content, players }: Props = $props();

  const segments = $derived(parseMentions(content));
</script>

{#each segments as seg}{#if "text" in seg}{seg.text}{:else}<Chip
      label={mentionLabel(seg.mention, players)}
      colorVar={mentionColorVar(seg.mention)}
    />{/if}{/each}
