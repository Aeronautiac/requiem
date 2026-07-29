<script lang="ts">
  // Always visible rather than behind a button: being in custody or under the radar changes what
  // you should be doing right now, so it is status, not a list.
  //
  // Own states only, which is the whole shape of ActorState — what you know about anyone ELSE
  // comes from the event that announced it, never from here.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import { StateFlag } from "../../bindings";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  // Ordered worst-first, so the badge that matters most reads first.
  const BADGES: { flag: number; label: string; classes: string }[] = [
    { flag: StateFlag.Dead, label: "Dead", classes: "bg-red-600/20 text-red-300" },
    { flag: StateFlag.Custody, label: "In Custody", classes: "bg-rose-600/20 text-rose-300" },
    { flag: StateFlag.Kidnapped, label: "Kidnapped", classes: "bg-amber-600/20 text-amber-300" },
    { flag: StateFlag.Incarcerated, label: "Imprisoned", classes: "bg-slate-500/20 text-slate-300" },
    { flag: StateFlag.UnderTheRadar, label: "Off the Record", classes: "bg-neutral-600/30 text-neutral-300" },
    { flag: StateFlag.Ipp, label: "IPP", classes: "bg-emerald-600/20 text-emerald-300" },
  ];

  const states = $derived(view.states ?? 0);
  const active = $derived(BADGES.filter((b) => (states & b.flag) !== 0));
</script>

{#each active as badge (badge.label)}
  <span class="rounded px-2 py-0.5 text-xs font-medium {badge.classes}">
    {badge.label}
  </span>
{/each}
