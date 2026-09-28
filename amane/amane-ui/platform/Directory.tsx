// The platform directory: every game currently known to the server, polled while this screen is
// mounted. Anyone can see it, logged in or not — it carries no per-account data.
import type { RosterEntry } from "amane-client/bindings.ts";
import type { Client } from "amane-client/client.ts";
import { useEffect, useState } from "react";
import { Button } from "../kit/Button.tsx";

export function Directory({
  client,
  onSelect,
}: {
  client: Client;
  // A row was clicked: fill the join form with this game id.
  onSelect: (gameId: number) => void;
}) {
  const [roster, setRoster] = useState<RosterEntry[] | null>(null);

  useEffect(() => {
    let live = true;
    async function poll() {
      const reply = await client.roster();
      if (live && reply.ok) setRoster(reply.value);
    }
    void poll();
    const id = setInterval(poll, 15000);
    return () => {
      live = false;
      clearInterval(id);
    };
  }, [client]);

  async function refresh() {
    const reply = await client.roster();
    if (reply.ok) setRoster(reply.value);
  }

  // On a wide screen the directory takes whatever height its column has left. On a phone it is
  // capped at about half the screen, so a long directory can't push everything below it out of
  // reach. Either way only its rows scroll.
  return (
    <section className="flex max-h-[50dvh] min-h-64 flex-col border border-edge bg-panel lg:max-h-none lg:min-h-0 lg:flex-1">
      <div className="flex shrink-0 items-center justify-between border-b border-edge px-3 py-2">
        <h2 className="text-xs uppercase tracking-wide text-ink-dim">Directory</h2>
        <Button size="sm" variant="ghost" onClick={() => void refresh()}>
          Refresh
        </Button>
      </div>
      {roster === null ? (
        <p className="p-3 text-sm text-ink-dim">…</p>
      ) : roster.length === 0 ? (
        <p className="p-3 text-sm text-ink-dim">No games running.</p>
      ) : (
        <ul className="min-h-0 flex-1 overflow-y-auto [&>li]:border-b [&>li]:border-edge">
          {roster.map((entry) => (
            <li key={entry.game_id}>
              <button
                type="button"
                className="flex w-full min-w-0 items-center justify-between gap-3 px-3 py-2 text-left hover:bg-raised"
                onClick={() => onSelect(entry.game_id)}
              >
                <span className="text-sm text-ink">Game {entry.game_id}</span>
                <span className="shrink-0 text-right text-xs text-ink-dim">
                  {entry.keys} {entry.keys === 1 ? "key" : "keys"} online · {entry.connections}{" "}
                  {entry.connections === 1 ? "connection" : "connections"} ·{" "}
                  {entry.resident ? "resident" : "hibernated"}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
