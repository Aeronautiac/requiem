<script lang="ts">
  import { setContext } from "svelte";
  import { createTransport } from "$lib/transport";
  import { ClientState, CLIENT_KEY } from "../client.svelte";
  import Channels from "./game/Channels.svelte";
  import Players from "./game/Players.svelte";
  import GcControls from "./game/GcControls.svelte";
  import OrgPanel from "./game/OrgPanel.svelte";
  import Polls from "./game/Polls.svelte";
  import Prosecutions from "./game/Prosecutions.svelte";
  import { GAME_STATE_KEY } from "../game_state.svelte";
  import { UI_STATE_KEY } from "../ui_state.svelte";
  import ViewSelect from "./game/ViewSelect.svelte";
  import AbilityMenu from "./game/abilities/AbilityMenu.svelte";
  import PassivesPanel from "./game/PassivesPanel.svelte";
  import ChannelView from "./game/ChannelView.svelte";
  import AdminPanel from "./admin/AdminPanel.svelte";

  const client = new ClientState(createTransport());
  setContext(CLIENT_KEY, client);
  setContext(GAME_STATE_KEY, client.game);
  setContext(UI_STATE_KEY, client.ui);
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
    <PassivesPanel />
    <AdminPanel />
  </div>
</div>
