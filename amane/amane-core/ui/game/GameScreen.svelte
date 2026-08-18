<script lang="ts">
  import { getContext, setContext, untrack } from "svelte";
  import { CLIENT_KEY, ClientState } from "../../client.svelte";
  import { SESSION_KEY, SessionState } from "../../session.svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { UI_STATE_KEY } from "../../ui_state.svelte";
  import Channels from "./Channels.svelte";
  import Players from "./Players.svelte";
  import GcControls from "./GcControls.svelte";
  import RightControls from "./RightControls.svelte";
  import ViewSelect from "./ViewSelect.svelte";
  import GameClock from "./GameClock.svelte";
  import AbilityMenu from "./abilities/AbilityMenu.svelte";
  import PassivesPanel from "./PassivesPanel.svelte";
  import StatusBadges from "./StatusBadges.svelte";
  import ChannelView from "./ChannelView.svelte";
  import AdminPanel from "../admin/AdminPanel.svelte";
  import PlayerMenu from "./PlayerMenu.svelte";
  import Button from "../kit/Button.svelte";
  import { PLAYER_MENU_KEY, PlayerMenuController } from "./player_menu.svelte";
  import { tooltip } from "../../lib/tooltip";

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
  setContext(PLAYER_MENU_KEY, new PlayerMenuController());

  // The client owns leaving: it closes the connection and drops back to the platform screen.
  const client = getContext<ClientState>(CLIENT_KEY);

  // Rail widths, in px. Drag the divider between a rail and the message column, or focus it and use
  // the arrow keys. Bounds keep either rail from swallowing the conversation or collapsing to nothing.
  const MIN = 160;
  const MAX = 480;
  let left_width = $state(208);
  let right_width = $state(224);
  const clamp = (v: number) => Math.min(MAX, Math.max(MIN, v));

  function drag(side: "left" | "right", e: PointerEvent) {
    e.preventDefault();
    const start_x = e.clientX;
    const start = side === "left" ? left_width : right_width;
    const move = (ev: PointerEvent) => {
      // The right rail grows as the pointer moves left, so its delta is inverted.
      const dx = side === "left" ? ev.clientX - start_x : start_x - ev.clientX;
      if (side === "left") left_width = clamp(start + dx);
      else right_width = clamp(start + dx);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function key_resize(side: "left" | "right", e: KeyboardEvent) {
    const step = e.key === "ArrowLeft" ? -16 : e.key === "ArrowRight" ? 16 : 0;
    if (step === 0) return;
    e.preventDefault();
    if (side === "left") left_width = clamp(left_width + step);
    else right_width = clamp(right_width - step);
  }
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
      <aside
        class="shrink-0 overflow-y-auto"
        style="width: {left_width}px"
      >
        <Channels />
      </aside>
      <button
        type="button"
        aria-label="Resize left panel"
        class="w-1 shrink-0 cursor-col-resize bg-neutral-800 hover:bg-neutral-600"
        onpointerdown={(e) => drag("left", e)}
        onkeydown={(e) => key_resize("left", e)}
      ></button>
      <main class="min-w-0 flex-1 overflow-hidden">
        <ChannelView />
      </main>
      <button
        type="button"
        aria-label="Resize right panel"
        class="w-1 shrink-0 cursor-col-resize bg-neutral-800 hover:bg-neutral-600"
        onpointerdown={(e) => drag("right", e)}
        onkeydown={(e) => key_resize("right", e)}
      ></button>
      <aside
        class="shrink-0 overflow-y-auto"
        style="width: {right_width}px"
      >
        <RightControls />
        <GcControls />
        <Players />
      </aside>
    </div>
    <div
      class="flex items-center gap-2 px-3 py-1.5 border-t border-neutral-800 shrink-0"
    >
      <ViewSelect />
      <AbilityMenu />
      <PassivesPanel />
      {#if self.administers}
        <AdminPanel />
      {/if}
<StatusBadges />
      <div class="ml-auto flex items-center gap-2">
        <GameClock />
        <span
          use:tooltip
          data-tip={ui.notifications_enabled
            ? "Notifications on — click to mute popups"
            : "Notifications muted — click to mute"}
        >
          <Button
            variant="ghost"
            size="sm"
            class={ui.notifications_enabled
              ? ""
              : "text-red-400/90 line-through hover:text-red-300"}
            onclick={() => (ui.notifications_enabled = !ui.notifications_enabled)}
          >
            {ui.notifications_enabled ? "Notifications" : "Muted"}
          </Button>
        </span>
        <span use:tooltip data-tip="Disconnect and return to the platform">
          <Button variant="ghost" size="sm" onclick={() => client.leave()}>
            Menu
          </Button>
        </span>
      </div>
    </div>
  </div>

  <PlayerMenu />
{/if}
