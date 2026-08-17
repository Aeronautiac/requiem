<script lang="ts">
  // One person row in the Players panel. The name is the trigger: clicking it opens the shared
  // profile menu (contact / conference / admin), so this row is just the name, its public statuses,
  // and the channel read/send hint — no inline dropdown of its own anymore.
  import { permsLabel, statusBadgeStyle, statusLabels } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import Name from "./Name.svelte";

  interface Props {
    id: string;
    // read/send hint for channel members; omit (null) for non-members.
    perms?: number | null;
  }
  let { id, perms = null }: Props = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  // The public condition of this player, as the world-data viewport last projected it.
  const statuses = $derived(statusLabels(view.actor_statuses.get(id) ?? 0));
</script>

<div class="flex items-start justify-between gap-2 px-2 py-1.5 text-sm">
  <span class="flex min-w-0 flex-wrap items-center gap-1.5">
    <Name {id} {view} />
    {#each statuses as s (s)}
      <span
        class="rounded px-1 py-px text-[0.6rem] uppercase tracking-wide"
        style={statusBadgeStyle(s)}
      >
        {s}
      </span>
    {/each}
  </span>
  {#if perms !== null && permsLabel(perms)}
    <span class="shrink-0 text-xs text-neutral-600">{permsLabel(perms)}</span>
  {/if}
</div>
