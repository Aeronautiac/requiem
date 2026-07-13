<script lang="ts">
  import { setContext } from "svelte";
  import { createTransport } from "$lib/transport";
  import Channels from "./game/Channels.svelte";
  import Players from "./game/Players.svelte";
  import GcControls from "./game/GcControls.svelte";
  import OrgPanel from "./game/OrgPanel.svelte";
  import Polls from "./game/Polls.svelte";
  import Prosecutions from "./game/Prosecutions.svelte";
  import { GAME_STATE_KEY, GameState } from "../game_state.svelte";
  import { UI_STATE_KEY, UiState } from "../ui_state.svelte";
  import ViewSelect from "./game/ViewSelect.svelte";
  import AbilityMenu from "./game/abilities/AbilityMenu.svelte";
  import ChannelView from "./game/ChannelView.svelte";
  import AdminPanel from "./admin/AdminPanel.svelte";

  const router = createTransport();
  const game_state = new GameState();
  game_state.attach(router);
  setContext(GAME_STATE_KEY, game_state);
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
      <Polls />
      <Prosecutions />
      <OrgPanel />
      <GcControls />
      <Players />
    </aside>
  </div>
  <div
    class="flex items-center gap-2 px-3 py-2 border-t border-neutral-800 shrink-0"
  >
    <ViewSelect />
    <AbilityMenu />
    <AdminPanel />
  </div>
</div>
