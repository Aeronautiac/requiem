<script lang="ts">
  // One ActorDisplay rendered as its chip, the single place a party/contact/mention resolves to
  // markup. A Raw player (and System) reads as a clickable Name — coloured and opens the profile
  // menu — while a Role, Org, or Mysterious display reads as a coloured Chip, because there is no
  // player behind it to click on. `menu` is off when the chip already sits inside its own clickable
  // row (see Name for why).
  import type { ActorDisplay } from "../../bindings";
  import { slotKeyToString } from "../../bindings";
  import { actorDisplayColor, actorDisplayLabel } from "../../game/helpers.svelte";
  import type { GameView } from "../../game/view.svelte";
  import Chip from "./Chip.svelte";
  import Name from "./Name.svelte";

  let { display, view, menu = true }: {
    display: ActorDisplay;
    view: GameView;
    menu?: boolean;
  } = $props();
</script>

{#if display === "Mysterious"}
  <Chip label={actorDisplayLabel(display, view)} colorVar={actorDisplayColor(display, view)} />
{:else if display === "System"}
  <Name id="System" {view} chip menu={false} />
{:else if "Raw" in display}
  <Name id={slotKeyToString(display.Raw)} {view} chip {menu} />
{:else}
  <Chip label={actorDisplayLabel(display, view)} colorVar={actorDisplayColor(display, view)} />
{/if}
