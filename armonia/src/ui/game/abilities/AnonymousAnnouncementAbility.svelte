<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let content = $state("");
  const flash = new Flash();

  async function announce() {
    if (!content.trim()) {
      flash.set_error("Write something to announce.");
      return;
    }
    const err = await game.dispatch(
      useAbilityRequest(ui.viewer, abilityId, {
        AnonymousAnnouncement: { content },
      }),
    );
    if (err) flash.set_error(err);
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    Broadcast an anonymous announcement to the news feed.
  </p>
  <textarea
    bind:value={content}
    rows="4"
    placeholder="Announcement…"
    class="w-full resize-none rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
  ></textarea>
  <button
    class="rounded-md bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-900 hover:bg-white"
    onclick={announce}
  >
    Announce
  </button>
  <FlashDisplay {flash} />
</div>
