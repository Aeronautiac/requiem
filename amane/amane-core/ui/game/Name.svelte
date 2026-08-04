<script lang="ts">
  // One actor's name, coloured by public position — the single place a name renders, so its colour
  // always reflects status (see nameColorVar: System, news anchor, press conference, else civilian).
  // Pass the view being rendered: the same key can hold a post in one view and not in another, so
  // colour is a property of the view, not a global.
  //
  // `chip` is the pill form (background + padding) that reads like a mention, for announcements where
  // a name is a prominent reference. The default is plain coloured text — a pill on every name (the
  // roster, inline lists) is too heavy.
  //
  // Clicking a player's name opens the shared profile menu (contact, conference, admin). `menu` is
  // off for names that already sit inside their own clickable row (an "add member" button), where a
  // nested button is both invalid and the wrong action.
  import type { GameView } from "../../game/view.svelte";
  import { nameColorVar, t } from "../../game/helpers.svelte";
  import Chip from "./Chip.svelte";
  import { getPlayerMenu } from "./player_menu.svelte";

  let {
    id,
    view,
    chip = false,
    menu = true,
  }: { id: string; view: GameView; chip?: boolean; menu?: boolean } = $props();

  const controller = getPlayerMenu();
  const label = $derived(id === "System" ? t("display_system") : view.actor_name(id));
  const color = $derived(nameColorVar(id, view));
  // Only a real player opens the menu; System and unknown keys are inert.
  const clickable = $derived(menu && controller != null && view.players.has(id));
</script>

{#snippet body()}{#if chip}<Chip
      {label}
      colorVar={color}
    />{:else}<span class="font-medium" style="color: {color}">{label}</span>{/if}{/snippet}

{#if clickable}<button
    type="button"
    class="cursor-pointer align-baseline hover:opacity-80"
    onclick={() => controller?.open(id)}>{@render body()}</button
  >{:else}{@render body()}{/if}
