// How React reads the core. The core is plain mutable data with one change signal, `version`, which
// moves once per batch; a component that reads state calls `useClient` and re-renders when it moves.
import type { Client } from "amane-client/client.ts";
import type { Session } from "amane-client/session.ts";
import { createContext, useCallback, useContext, useEffect, useState, useSyncExternalStore } from "react";

export const ClientContext = createContext<Client | null>(null);

export function useClient(): Client {
  const client = useContext(ClientContext);
  if (!client) throw new Error("useClient outside <App>");
  const subscribe = useCallback((listener: () => void) => client.subscribe(listener), [client]);
  // The snapshot is the version NUMBER. Returning an object built here would differ on every call,
  // and React would re-render forever.
  useSyncExternalStore(subscribe, () => client.version);
  return client;
}

// The joined session. Only for components under the game screen, which renders only with one.
export function useSession(): Session {
  const session = useClient().session;
  if (!session) throw new Error("useSession outside the game screen");
  return session;
}

// Whether a media query matches, followed live. For layout that differs in STRUCTURE by screen,
// where CSS alone would have to mount both versions (and both copies' dialogs and state).
export function useMedia(query: string): boolean {
  const subscribe = useCallback(
    (listener: () => void) => {
      const list = matchMedia(query);
      list.addEventListener("change", listener);
      return () => list.removeEventListener("change", listener);
    },
    [query],
  );
  return useSyncExternalStore(subscribe, () => matchMedia(query).matches);
}

// Pin the app over the part of the screen actually visible: its height and where it starts, as
// `--app-height` and `--app-top` (read in app.css). When a phone keyboard opens, iOS both shrinks
// the visible area and slides it down the page to reveal the focused box, whatever the viewport
// meta asks, and nothing undoes the slide. So the app follows it instead: the composer sits on the
// keyboard and nothing above it moves. A pinch-zoom moves the visible area too; the app is left
// alone while zoomed, or zooming in would chase itself.
export function useFitVisualViewport() {
  useEffect(() => {
    const visual = window.visualViewport;
    if (!visual) return;
    const root = document.documentElement.style;
    const fit = () => {
      if (visual.scale > 1.01) return;
      root.setProperty("--app-height", `${visual.height}px`);
      root.setProperty("--app-top", `${visual.offsetTop}px`);
    };
    fit();
    visual.addEventListener("resize", fit);
    visual.addEventListener("scroll", fit);
    return () => {
      visual.removeEventListener("resize", fit);
      visual.removeEventListener("scroll", fit);
    };
  }, []);
}

// The current time, re-read every `ms`. For what moves with the clock rather than with state: the
// game clock, and death beats whose moment arrives with nothing new delivered.
export function useNow(ms: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), ms);
    return () => clearInterval(id);
  }, [ms]);
  return now;
}
