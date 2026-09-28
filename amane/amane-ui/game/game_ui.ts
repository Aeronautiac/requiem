// The game screen's own state: what is selected and open. Lives in React, not the core, and dies
// with the GameScreen, which is keyed by session so a new session starts with none of it.
import type { View } from "amane-client/game/view.ts";
import { createContext, useContext } from "react";
import { useSession } from "../hooks.ts";

// News is not a channel: it exists on its own and renders the world feed.
export type Selection = { kind: "news" } | { kind: "channel"; id: string };

// The widget strip under the channel header. One at a time; the button that opens it closes it.
export type TopPanel = "polls" | "prosecutions";

export type GameUi = {
  // The view on screen, by view key ("System" for admin).
  viewer: string;
  setViewer: (viewer: string) => void;
  selected: Selection | null;
  select: (selection: Selection | null) => void;
  top_panel: TopPanel | null;
  togglePanel: (panel: TopPanel) => void;
  // A poll the open channel should scroll to; the channel view clears it once it has.
  jump_poll: string | null;
  setJumpPoll: (poll: string | null) => void;
  // Whether OS notifications are raised. The in-app Notifications feed is unaffected.
  notifications: boolean;
  setNotifications: (on: boolean) => void;
  // The player whose profile menu is open. One menu for the whole screen; every name opens it.
  player_menu: string | null;
  openPlayerMenu: (player: string | null) => void;
  // Narrow screens only: which side drawer is out.
  drawer: "left" | "right" | null;
  setDrawer: (drawer: "left" | "right" | null) => void;
};

export const GameUiContext = createContext<GameUi | null>(null);

export function useGameUi(): GameUi {
  const ui = useContext(GameUiContext);
  if (!ui) throw new Error("useGameUi outside the game screen");
  return ui;
}

// The view on screen. Everything a game component shows comes from here.
export function useView(): View {
  const session = useSession();
  const { viewer } = useGameUi();
  return session.game.views.get(viewer) ?? session.game.system_view();
}
