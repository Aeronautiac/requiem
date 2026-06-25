<script lang="ts">
  import { setContext } from "svelte";
  import { createTransport } from "$lib/transport";
  import { ROUTER_KEY } from "$lib/router";
  import AddPlayers from "./AddPlayers.svelte";
  import Channels from "./Channels.svelte";
  import Players from "./Players.svelte";
  import { GAME_STATE_KEY, GameState } from "./game_state.svelte";
  import { UI_STATE_KEY, UiState } from "./ui_state.svelte";
  import ViewSelect from "./ViewSelect.svelte";
  import ChannelView from "./ChannelView.svelte";

  setContext(ROUTER_KEY, createTransport());
  setContext(GAME_STATE_KEY, new GameState());
  setContext(UI_STATE_KEY, new UiState());
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
  </div>
</div>
