// What shows when no game is joined: the account (log in or who you are), a way to join a game by
// id and key, and — for a logged-in account — its saved keys, its own games, and the directory of
// everything running on the server.
import type { Client } from "amane-client/client.ts";
import { STRINGS } from "amane-client/strings.ts";
import { execErrorText } from "amane-client/text.ts";
import type { FormEvent } from "react";
import { useRef, useState } from "react";
import { useClient } from "../hooks.ts";
import { Button } from "../kit/Button.tsx";
import { ErrorBoundary } from "../kit/ErrorBoundary.tsx";
import { Input } from "../kit/Input.tsx";
import { AccountBox } from "./AccountBox.tsx";
import { Directory } from "./Directory.tsx";
import { GamesPanel } from "./GamesPanel.tsx";

export function Platform() {
  const client = useClient();
  // One splash, picked once on mount rather than on every render.
  const [splash] = useState(
    () => STRINGS.platform_splashes[Math.floor(Math.random() * STRINGS.platform_splashes.length)],
  );
  const [gameId, setGameId] = useState("");
  const [key, setKey] = useState("");
  const keyField = useRef<HTMLInputElement>(null);

  // A failed join lands in `client.phase`, which the banner shows, so the reply isn't read here.
  function submitJoin(e: FormEvent) {
    e.preventDefault();
    const id = Number(gameId);
    if (!Number.isInteger(id) || id < 0 || key.trim() === "") return;
    void client.join(id, key.trim());
  }

  function selectFromDirectory(id: number) {
    setGameId(String(id));
    setKey("");
    keyField.current?.focus();
  }

  // A phone gets one column and the page scrolls as a whole.
  //
  // A wide screen gets a fixed frame instead: the header, then two columns filling the rest of the
  // screen (capped so a tall monitor doesn't stretch it, and centred by `my-auto`). Nothing in it
  // grows or shifts: the directory and the games card each take the space left in their column,
  // and only their lists scroll. The right column is there for a guest too, so logging in fills a
  // space rather than moving everything.
  //
  // The frame also has a floor: on a wide but short window it stops shrinking there and the page
  // scrolls as a whole, rather than squeezing the lists to nothing and cutting off the bottom.
  return (
    <div className="flex h-full flex-col overflow-y-auto p-4">
      <div className="mx-auto my-auto flex w-full max-w-5xl flex-col gap-6 lg:h-full lg:max-h-[56rem] lg:min-h-[40rem]">
        <header className="shrink-0 space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight text-ink">requiem-dn</h1>
          <p className="text-sm text-ink-dim">{splash}</p>
        </header>

        <PhaseBanner client={client} />

        <div className="grid gap-6 lg:min-h-0 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_minmax(16rem,22rem)] lg:grid-rows-1">
          <div className="flex min-w-0 flex-col gap-6 lg:min-h-0">
            <ErrorBoundary name="Account">
              <AccountBox client={client} />
            </ErrorBoundary>

            <ErrorBoundary name="Join a game">
              <section className="flex flex-col gap-2 border border-edge bg-panel p-3">
                <h2 className="text-xs uppercase tracking-wide text-ink-dim">Join a game</h2>
                <form className="flex flex-col gap-2 sm:flex-row" onSubmit={submitJoin}>
                  <Input
                    className="w-full sm:w-24"
                    inputMode="numeric"
                    placeholder="Game id"
                    value={gameId}
                    onChange={(e) => setGameId(e.target.value)}
                  />
                  <Input
                    ref={keyField}
                    className="w-full sm:flex-1"
                    placeholder="Key"
                    value={key}
                    onChange={(e) => setKey(e.target.value)}
                    autoComplete="off"
                  />
                  <Button
                    type="submit"
                    disabled={client.phase.status === "joining" || gameId.trim() === "" || key.trim() === ""}
                  >
                    {client.phase.status === "joining" ? "Joining…" : "Join"}
                  </Button>
                </form>
              </section>
            </ErrorBoundary>

            <ErrorBoundary name="Directory">
              <Directory client={client} onSelect={selectFromDirectory} />
            </ErrorBoundary>
          </div>

          <aside className="flex min-w-0 flex-col lg:min-h-0">
            {client.account.kind === "member" ? (
              <ErrorBoundary name="Your games">
                <GamesPanel
                  client={client}
                  packet={client.account.packet}
                  onJoin={(id, joinKey) => void client.join(id, joinKey)}
                />
              </ErrorBoundary>
            ) : (
              <section className="flex min-h-40 flex-col items-center justify-center gap-1 border border-edge bg-panel p-6 text-center lg:min-h-0 lg:flex-1">
                <p className="text-sm text-ink">Your games</p>
                <p className="text-sm text-ink-dim">Log in to create games and keep the keys you join with.</p>
              </section>
            )}
          </aside>
        </div>
      </div>
    </div>
  );
}

function PhaseBanner({ client }: { client: Client }) {
  const phase = client.phase;
  if (phase.status === "joining") {
    return <p className="border border-edge bg-panel px-3 py-2 text-sm text-ink-dim">Connecting…</p>;
  }
  if (phase.status === "failed" || phase.status === "dropped") {
    const text = phase.status === "failed" ? execErrorText(phase.error) : phase.reason;
    return (
      <div className="flex items-center gap-2 border border-edge bg-panel px-3 py-2 text-sm text-danger-text">
        <p className="min-w-0 flex-1">{text}</p>
        <Button size="sm" variant="ghost" onClick={() => client.dismiss()}>
          Dismiss
        </Button>
      </div>
    );
  }
  return null;
}
