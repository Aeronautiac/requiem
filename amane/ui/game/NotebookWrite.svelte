<script lang="ts">
  import { getContext } from "svelte";
  import Dialog from "$lib/components/ui/dialog/dialog.svelte";
  import DialogContent from "$lib/components/ui/dialog/dialog-content.svelte";
  import DialogHeader from "$lib/components/ui/dialog/dialog-header.svelte";
  import DialogTitle from "$lib/components/ui/dialog/dialog-title.svelte";
  import Input from "$lib/components/ui/input/input.svelte";
  import { Button } from "$lib/components/ui/button";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest, NotebookKey } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  interface Props {
    open: boolean;
    notebookId: NotebookKey;
  }
  let { open = $bindable(), notebookId }: Props = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let true_name = $state("");
  let death_message = $state("");
  let seconds = $state(0);
  let minutes = $state(0);
  let hours = $state(0);
  let days = $state(0);
  const flash = new Flash();

  // delay is a duration in milliseconds (backend adds it to the current engine
  // time). Coerce each field defensively: a cleared number input can yield null.
  const delay_ms = $derived(
    ((((Number(days) || 0) * 24 + (Number(hours) || 0)) * 60 +
      (Number(minutes) || 0)) *
      60 +
      (Number(seconds) || 0)) *
      1000,
  );

  function reset() {
    true_name = "";
    death_message = "";
    seconds = 0;
    minutes = 0;
    hours = 0;
    days = 0;
  }

  async function submit() {
    if (!true_name.trim()) {
      flash.set_error("A true name is required.");
      return;
    }
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        WriteName: {
          true_name: true_name.trim(),
          death_message: death_message.trim() ? death_message.trim() : null,
          notebook_id: notebookId,
          delay: delay_ms,
        },
      },
    };
    const err = await client.dispatch(request);
    if (err) {
      flash.set_error(`Write failed: ${err}`);
    } else {
      flash.set_success("Name written.");
      reset();
    }
  }
</script>

<Dialog bind:open>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Write Name</DialogTitle>
    </DialogHeader>

    <Input bind:value={true_name} placeholder="True Name" />
    <Input bind:value={death_message} placeholder="Death Message (optional)" />

    <div>
      <p class="mb-1 text-xs text-neutral-400">Delay</p>
      <div class="grid grid-cols-4 gap-2">
        <label class="flex flex-col gap-1 text-xs text-neutral-400">
          Seconds
          <Input type="number" min="0" bind:value={seconds} />
        </label>
        <label class="flex flex-col gap-1 text-xs text-neutral-400">
          Minutes
          <Input type="number" min="0" bind:value={minutes} />
        </label>
        <label class="flex flex-col gap-1 text-xs text-neutral-400">
          Hours
          <Input type="number" min="0" bind:value={hours} />
        </label>
        <label class="flex flex-col gap-1 text-xs text-neutral-400">
          Days
          <Input type="number" min="0" bind:value={days} />
        </label>
      </div>
    </div>

    <Button onclick={submit}>Write</Button>
    <FlashDisplay {flash} />
  </DialogContent>
</Dialog>
