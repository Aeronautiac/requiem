<script lang="ts">
  import { setContext } from "svelte";
  import { createTransport } from "$lib/transport";
  import { ROUTER_KEY } from "$lib/router";
  import AddPlayers from "./admin/AddPlayers.svelte";
  import Channels from "./game/Channels.svelte";
  import Players from "./game/Players.svelte";
  import { GAME_STATE_KEY, GameState } from "../game_state.svelte";
  import { UI_STATE_KEY, UiState } from "../ui_state.svelte";
  import ViewSelect from "./game/ViewSelect.svelte";
  import ChannelView from "./game/ChannelView.svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import type { ActionRequest } from "../bindings";
  import { Flash } from "../flash.svelte.ts";
  import FlashDisplay from "./Flash.svelte";

  const router = createTransport();
  const game_state = new GameState();
  setContext(ROUTER_KEY, router);
  setContext(GAME_STATE_KEY, game_state);
  setContext(UI_STATE_KEY, new UiState());
  const flash = new Flash();

  async function init() {
    const request: ActionRequest = {
      actor: "Admin", // later enforce who is allowed to do this on the server
      timestamp: Date.now(),
      payload: {
        InitializeEngine: {
          seed: 0,
        },
      },
    };

    const response = await router.sendAction(request);
    console.log(response);
    const err = game_state.process_response(response);
    if (err) {
      flash.set_error(`Action Failed: ${err}`);
    } else {
      flash.set_success(`Successfully initialized Engine`);
    }
  }
</script>

<div class="flex flex-col h-screen bg-neutral-950 text-white">
  <div class="flex flex-1 overflow-hidden">
    <aside class="w-52 shrink-0 border-r border-neutral-800 overflow-y-auto">
      <Channels />
    </aside>
    <main class="flex-1 overflow-hidden">
      <ChannelView />
    </main>
    <aside class="w-52 shrink-0 border-l border-neutral-800 overflow-y-auto">
      <Players />
    </aside>
  </div>
  <div
    class="flex items-center gap-2 px-3 py-2 border-t border-neutral-800 shrink-0"
  >
    <ViewSelect />
    <AddPlayers />
    <div>
      <Button
        onclick={async () => {
          await init();
        }}>Initialize Engine</Button
      >
      <FlashDisplay {flash} />
    </div>
  </div>
</div>
