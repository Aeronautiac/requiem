<script lang="ts">
  // Pass (lend) a notebook to another player. Silent — it just dispatches LendNotebook; no
  // logging or channel event. Mirrors the NotebookWrite modal, opened from the notebook channel.
  import { getContext } from "svelte";
  import Dialog from "$lib/components/ui/dialog/dialog.svelte";
  import DialogContent from "$lib/components/ui/dialog/dialog-content.svelte";
  import DialogHeader from "$lib/components/ui/dialog/dialog-header.svelte";
  import DialogTitle from "$lib/components/ui/dialog/dialog-title.svelte";
  import { Button } from "$lib/components/ui/button";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
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

  const client = getContext<ClientState>(CLIENT_KEY);
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
    const err = await client.dispatch(request);
    if (err) {
      flash.set_error(`Pass failed: ${err}`);
    } else {
      flash.set_success("Notebook passed.");
      target = "";
      open = false;
    }
  }
</script>

<Dialog bind:open>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Pass Notebook</DialogTitle>
    </DialogHeader>

    <PlayerSelect bind:value={target} placeholder="Pass to" />

    <Button onclick={submit}>Pass</Button>
    <FlashDisplay {flash} />
  </DialogContent>
</Dialog>
