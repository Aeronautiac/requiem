<script lang="ts">
  // One ping chip: a name/role/org rendered as a coloured inline pill. The accent comes in as a
  // `var(...)` reference (see mentionColorVar / roleColorVar), so recolouring is a theme.css edit
  // and every chip — in a message, in the composer, on a death — reads from the same tokens.
  import { chipStyle } from "../../game/helpers.svelte";

  let { label, colorVar }: { label: string; colorVar: string } = $props();
</script>

<span
  class="box-decoration-clone rounded px-0.5 py-0.5 font-medium"
  style={chipStyle(colorVar)}>{label}</span
>
