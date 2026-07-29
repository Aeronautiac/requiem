<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { playerLabel } from "../../game/helpers.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import Select from "../kit/Select.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  // Which views exist is the session's answer; this only names them. System is what "Admin"
  // selects, so it is never offered under its own name.
  const options = $derived(
    session.viewers.map((key) => ({
      value: key,
      label: key === "Admin" ? "Admin" : playerLabel(key, view.players),
    })),
  );

  // The selected viewer is always one of the offered views. This is the ONLY rule deciding it, and
  // stating it here — where the options are built — is what keeps "Admin" from being reachable by a
  // key that does not administer: it is not in the list, so it cannot be held. It also covers a key
  // narrowed mid-game out of the actor it was watching. The list is never empty; the game screen
  // does not render at all until it has something in it.
  $effect(() => {
    if (!options.some((o) => o.value === ui.viewer)) ui.viewer = options[0].value;
  });
</script>

<Select bind:value={ui.viewer} {options} class="h-8" />
