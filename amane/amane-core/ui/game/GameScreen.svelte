<script lang="ts">
  import { setContext, untrack } from "svelte";
  import { SESSION_KEY, SessionState } from "../../session.svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte";
  import Channels from "./Channels.svelte";
  import Players from "./Players.svelte";
  import GcControls from "./GcControls.svelte";
  import OrgPanel from "./OrgPanel.svelte";
  import Polls from "./Polls.svelte";
  import Prosecutions from "./Prosecutions.svelte";
  import ViewSelect from "./ViewSelect.svelte";
  import AbilityMenu from "./abilities/AbilityMenu.svelte";
  import PassivesPanel from "./PassivesPanel.svelte";
  import StatusBadges from "./StatusBadges.svelte";
  import ChannelView from "./ChannelView.svelte";
  import AdminPanel from "../admin/AdminPanel.svelte";

  // The caller keys on this, so the component is rebuilt whenever it changes and the contexts
  // below can never outlive the session they came from.
  const { session }: { session: SessionState } = $props();

  // Reading the CURRENT values once at setup is the intent: context is set once per component
  // instance and cannot be reassigned. Without untrack Svelte flags these as accidentally
  // capturing an initial value.
  const { self, game, ui } = untrack(() => ({
    self: session,
    game: session.game,
    ui: session.ui,
  }));

  setContext(SESSION_KEY, self);
  setContext(GAME_STATE_KEY, game);
  setContext(UI_STATE_KEY, ui);
</script>

{#if self.viewers.length === 0}
  <!-- Nothing has been delivered to this connection yet, so there is no view to render and almost
       everything below would be reading state that does not exist. -->
  <p class="flex h-full items-center justify-center p-8 text-sm text-ink-dim">
    Waiting for the server.
  </p>
{:else}
<div class="flex flex-col h-full">
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
    {#if self.administers}
      <AdminPanel />
    {/if}
    <StatusBadges />
  </div>
</div>
{/if}
