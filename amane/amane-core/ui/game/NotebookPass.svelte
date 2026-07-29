<script lang="ts">
  // Pass (lend) a notebook to another player. Silent — it just dispatches LendNotebook; no
  // logging or channel event. Mirrors the NotebookWrite modal, opened from the notebook channel.
  import { execErrorText } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import Dialog from "../kit/Dialog.svelte";
  import Button from "../kit/Button.svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest, NotebookKey } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import PlayerSelect from "./abilities/PlayerSelect.svelte";

  interface Props {
    open: boolean;
    notebookId: NotebookKey;
  }
  let { open = $bindable(), notebookId }: Props = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let target = $state("");
  const flash = new Flash();

  async function submit() {
    if (!target) {
      flash.set_error("Pick a player to pass to.");
      return;
    }
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        LendNotebook: {
          notebook_id: notebookId,
          target_id: slotKeyFromString(target),
        },
      },
    };
    const reply = await session.submit_action(request);
    if (!reply.ok) {
      flash.set_error(`Pass failed: ${execErrorText(reply.error)}`);
    } else {
      flash.set_success("Notebook passed.");
      target = "";
      open = false;
    }
  }
</script>

<Dialog bind:open title="Pass Notebook">
  <PlayerSelect bind:value={target} placeholder="Pass to" />

  <Button onclick={submit}>Pass</Button>
  <FlashDisplay {flash} />
</Dialog>
