// The root of the React surface. A host builds the Client and mounts this with it.
import type { Client } from "amane-client/client.ts";
import { ClientContext, useClient } from "./hooks.ts";
import { ErrorBoundary } from "./kit/ErrorBoundary.tsx";
import { GameScreen } from "./game/GameScreen.tsx";
import { Platform } from "./platform/Platform.tsx";

export function App({ client }: { client: Client }) {
  return (
    <ClientContext.Provider value={client}>
      <Root />
    </ClientContext.Provider>
  );
}

function Root() {
  const client = useClient();
  if (client.session) {
    // Keyed by session: a new session mounts a new tree, and every selection, draft and open
    // panel of the old one goes with the old tree. Nothing is reset by hand.
    return (
      <ErrorBoundary name="Game">
        <GameScreen key={client.session.id} />
      </ErrorBoundary>
    );
  }
  return (
    <ErrorBoundary name="Menu">
      <Platform />
    </ErrorBoundary>
  );
}
