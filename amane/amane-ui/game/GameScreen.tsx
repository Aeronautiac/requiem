// The screen for one joined game. Keyed by session in App, so every piece of state below dies with
// the session it belonged to.
//
// Layout: on a wide screen, three columns (channels | the open channel | people and controls), each
// rail sized by dragging its inner edge. Below `lg`, only the open channel, with the two rails as drawers
// behind buttons in its header. Nothing scrolls but the regions meant to: each rail and the
// message list scroll on their own inside a fixed frame.
import { useEffect, useRef, useState } from "react";
import type { Delivered } from "amane-client/game/game.ts";
import { administers, viewers } from "amane-client/queries/session.ts";
import { formatTime, playerLabel } from "amane-client/text.ts";
import { useClient, useMedia, useNow, useSession } from "../hooks.ts";
import { Button } from "../kit/Button.tsx";
import { Drawer } from "../kit/Drawer.tsx";
import { ErrorBoundary } from "../kit/ErrorBoundary.tsx";
import { Select } from "../kit/Input.tsx";
import { Modal } from "../kit/Modal.tsx";
import { ResizableRail } from "../kit/ResizableRail.tsx";
import { notificationObserver } from "../notifications.ts";
import { AbilityMenu } from "./abilities/AbilityMenu.tsx";
import { PassivesPanel } from "./abilities/PassivesPanel.tsx";
import { AdminPanel } from "./admin/AdminPanel.tsx";
import { ChannelList } from "./channels/ChannelList.tsx";
import { ChannelView } from "./channels/ChannelView.tsx";
import type { GameUi, Selection, TopPanel } from "./game_ui.ts";
import { GameUiContext, useGameUi, useView } from "./game_ui.ts";
import { PlayerMenu } from "./side/PlayerMenu.tsx";
import { SidePanel } from "./side/SidePanel.tsx";
import { StatusBadges } from "./side/StatusBadges.tsx";

export function GameScreen() {
  const client = useClient();
  const session = useSession();
  const offered = viewers(session);

  const [viewer, setViewer] = useState(offered[0] ?? "System");
  const [selected, select] = useState<Selection | null>(null);
  const [top_panel, setTopPanel] = useState<TopPanel | null>(null);
  const [jump_poll, setJumpPoll] = useState<string | null>(null);
  const [notifications, setNotifications] = useState(true);
  const [player_menu, openPlayerMenu] = useState<string | null>(null);
  const [drawer, setDrawer] = useState<"left" | "right" | null>(null);

  // The selected viewer is always one of the offered views. This is the only rule deciding it: a
  // key narrowed mid-game out of the actor it was watching falls back to the first it still has.
  const current = offered.includes(viewer) ? viewer : offered[0];
  useEffect(() => {
    if (current !== undefined && current !== viewer) setViewer(current);
  }, [current, viewer]);

  // OS notifications. The observer judges each delivery as it is folded; it reads the latest
  // screen state through a ref, since it is registered once and outlives renders.
  const screen = useRef({ viewer: current, notifications });
  screen.current = { viewer: current, notifications };
  useEffect(
    () =>
      client.observe(
        notificationObserver(
          (delivered: Delivered) =>
            !delivered.replayed &&
            screen.current.notifications &&
            delivered.view === screen.current.viewer,
          (toast) => void client.host.notify(toast),
        ),
      ),
    [client],
  );

  if (current === undefined) {
    // Nothing has been delivered to this connection yet, so there is no view to show.
    return (
      <p className="flex h-full items-center justify-center p-8 text-sm text-ink-dim">
        Waiting for the server.
      </p>
    );
  }

  const ui: GameUi = {
    viewer: current,
    setViewer,
    selected,
    select: (selection) => {
      select(selection);
      setDrawer(null);
    },
    top_panel,
    togglePanel: (panel) => setTopPanel(top_panel === panel ? null : panel),
    jump_poll,
    setJumpPoll,
    notifications,
    setNotifications,
    player_menu,
    openPlayerMenu,
    drawer,
    setDrawer,
  };

  const left = (
    <ErrorBoundary name="Channels">
      <ChannelList />
    </ErrorBoundary>
  );
  const right = (
    <ErrorBoundary name="Side panel">
      <SidePanel />
    </ErrorBoundary>
  );

  return (
    <GameUiContext.Provider value={ui}>
      <div className="flex h-full flex-col">
        <FaultBanner />
        <div className="flex min-h-0 flex-1">
          <ResizableRail side="left" name="channels" initial={256} min={192} max={480} className="hidden lg:block">
            {left}
          </ResizableRail>
          <main className="flex min-w-0 flex-1 flex-col">
            <ErrorBoundary name="Channel">
              <ChannelView />
            </ErrorBoundary>
          </main>
          <ResizableRail side="right" name="side" initial={288} min={208} max={560} className="hidden lg:block">
            {right}
          </ResizableRail>
        </div>
        <BottomBar />
      </div>

      <div className="lg:hidden">
        <Drawer side="left" open={drawer === "left"} onClose={() => setDrawer(null)}>
          <div className="min-h-0 flex-1 overflow-y-auto">{left}</div>
        </Drawer>
        <Drawer side="right" open={drawer === "right"} onClose={() => setDrawer(null)}>
          <div className="min-h-0 flex-1 overflow-y-auto">{right}</div>
        </Drawer>
      </div>

      <ErrorBoundary name="Player menu">
        <PlayerMenu />
      </ErrorBoundary>
    </GameUiContext.Provider>
  );
}

// On a wide screen, everything in one bar. On a phone, one row of what is used mid-play (abilities,
// passives, your own statuses, the clock), with the rest behind "More": a bar holding all of it
// would wrap into rows and eat the screen.
function BottomBar() {
  const client = useClient();
  const session = useSession();
  const ui = useGameUi();
  const wide = useMedia("(min-width: 64rem)"); // Tailwind's `lg`, where the rails show
  const [more, setMore] = useState(false);

  const admin = administers(session) && (
    <ErrorBoundary name="Admin">
      <AdminPanel />
    </ErrorBoundary>
  );
  const settings = (
    <>
      <Button
        variant="ghost"
        size="sm"
        title={ui.notifications ? "Notifications on: click to mute popups" : "Notifications muted"}
        className={ui.notifications ? "" : "text-danger-text line-through"}
        onClick={() => ui.setNotifications(!ui.notifications)}
      >
        {ui.notifications ? "Notifications" : "Muted"}
      </Button>
      <Button variant="ghost" size="sm" title="Disconnect and return to the menu" onClick={() => client.leave()}>
        Menu
      </Button>
    </>
  );

  return (
    <div className="flex shrink-0 items-center gap-2 border-t border-edge px-3 py-1.5 lg:flex-wrap">
      {wide && <ViewSelect />}
      <ErrorBoundary name="Abilities">
        <AbilityMenu />
      </ErrorBoundary>
      <ErrorBoundary name="Passives">
        <PassivesPanel />
      </ErrorBoundary>
      {wide && admin}
      <div className="min-w-0">
        <StatusBadges />
      </div>
      <div className="ml-auto flex shrink-0 items-center gap-2">
        <GameClock />
        {wide ? (
          settings
        ) : (
          <Button variant="ghost" size="sm" onClick={() => setMore(true)}>
            More
          </Button>
        )}
      </div>

      {!wide && (
        <Modal open={more} onClose={() => setMore(false)} title="Game">
          <div className="flex flex-col gap-4">
            {viewers(session).length > 1 && (
              <label className="flex flex-col gap-1 text-xs text-ink-dim">
                Viewing as
                <ViewSelect />
              </label>
            )}
            {admin && (
              <div className="flex flex-col gap-1 text-xs text-ink-dim">
                Admin
                {admin}
              </div>
            )}
            <div className="flex flex-wrap gap-2">{settings}</div>
          </div>
        </Modal>
      )}
    </div>
  );
}

// Which view is on screen. The options are exactly the views this connection may look through.
function ViewSelect() {
  const session = useSession();
  const ui = useGameUi();
  // Each actor is named as its own view knows it.
  const options = viewers(session).map((key) => ({
    value: key,
    label: key === "System" ? "Admin" : playerLabel(key, session.game.views.get(key)!.players),
  }));
  return <Select value={ui.viewer} options={options} onChange={(event) => ui.setViewer(event.target.value)} className="w-auto" />;
}

// The current game time, in the same units as every timestamp. Re-read on its own tick: game time
// moves with the wall clock, with nothing delivered.
function GameClock() {
  useNow(250);
  const view = useView();
  return (
    <span className="flex h-8 items-center border border-edge bg-panel px-3 text-sm tabular-nums text-ink-dim">
      {formatTime(view.game_time_now())}
    </span>
  );
}

// The fold skipped something this view was sent, so part of it may be wrong. Persistent: it stays
// until a resync builds a fresh game.
function FaultBanner() {
  const session = useSession();
  const [open, setOpen] = useState(false);
  const faults = session.game.faults;
  if (faults.length === 0) return null;
  return (
    <div className="shrink-0 border-b border-edge bg-panel px-3 py-2 text-sm">
      <button type="button" className="text-left text-danger-text" onClick={() => setOpen(!open)}>
        Part of what you're seeing may be wrong: {faults.length} update{faults.length === 1 ? "" : "s"} failed to apply.{" "}
        <span className="text-ink-dim underline">{open ? "Hide" : "Details"}</span>
      </button>
      {open && (
        <ul className="mt-2 max-h-40 overflow-y-auto text-xs text-ink-dim">
          {faults.map((fault, i) => (
            <li key={i}>
              {formatTime(fault.time)} · {fault.name} → {fault.view ?? "routing"}: {fault.error}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
