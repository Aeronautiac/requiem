<script lang="ts">
  import { getContext } from "svelte";
  import Input from "$lib/components/ui/input/input.svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import { ROUTER_KEY } from "$lib/router";
  import type { Router } from "$lib/router";
  import type { ActionRequest, ActorDisplay } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import Button from "$lib/components/ui/button/button.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const router = getContext<Router>(ROUTER_KEY);

  let message_content = $state("");
  let channel_name = $derived(get_channel_name());

  function get_channel_name(): string | null {
    return (
      game.admin_view().channels.get(ui.selected_channel ?? "")?.name ?? null
    );
  }

  function sender_display(): ActorDisplay {
    if (ui.viewer === "Admin") return "System";
    return { Raw: slotKeyFromString(ui.viewer) };
  }

  function display_string(display: ActorDisplay): string {
    if (display === "Mysterious") return "???";
    if (display === "System") return "System";
    if ("Raw" in display)
      return (
        game.players.get(slotKeyToString(display.Raw))?.display_name ??
        "Unknown"
      );
    if ("Role" in display) return display.Role;
    if ("Org" in display) return "Org";
    return "Unknown";
  }

  async function send_message() {
    if (!ui.selected_channel || !message_content.trim()) return;
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: Date.now(),
      payload: {
        SendMessage: {
          channel_id: slotKeyFromString(ui.selected_channel),
          display: sender_display(),
          content: message_content.trim(),
        },
      },
    };
    game.process_response(await router.sendAction(request));
    message_content = "";
    console.log("message sent");
  }

  // only render messages sent before cutoff time (if applicable).
  // a channel view has a send text box, a channel name bar, a player list showing all members
  // in the channel, and a loggable toggle button (if you have the permission)
</script>

<div class="grid grid-rows-[auto_1fr_auto] h-full w-full">
  {#if ui.selected_channel}
    <header class="flex h-14 z-50 flex">
      <div>{channel_name}</div>
    </header>
    <main class="flex-grow">messages here</main>
    <footer class="flex h-14 items-center z-50 flex">
      <form
        onsubmit={async () => {
          await send_message();
        }}
      >
        <div class="flex">
          <Input bind:value={message_content} placeholder="Message Content" />
          <Button onclick={() => {}}>Send</Button>
        </div>
      </form>
      <Button onclick={() => {}}>Loggable</Button>
    </footer>
  {:else}
    Select a channel
  {/if}
</div>
