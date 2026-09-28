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
