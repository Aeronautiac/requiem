// Everything an account's own games need: creating one, the games it created (and may end), and
// the keys it saved by joining. Member-only.
//
// The two lists share one card behind tabs rather than stacking, so neither pushes the other
// around. On a wide screen the card fills the rest of its column and only the list inside scrolls.
import type { AccountKey, AccountPacket, GameCreationPacket } from "amane-client/bindings.ts";
import { privilegesLabel } from "amane-client/text.ts";
import type { Client } from "amane-client/client.ts";
import type { ReactNode } from "react";
import { useState } from "react";
import { Button } from "../kit/Button.tsx";
import { copyToClipboard } from "../kit/clipboard.ts";
import { ConfirmButton } from "../kit/ConfirmButton.tsx";
import { FlashLine, useFlash } from "../kit/Flash.tsx";
import { Select } from "../kit/Input.tsx";
import { Modal } from "../kit/Modal.tsx";

export function GamesPanel({
  client,
  packet,
  onJoin,
}: {
  client: Client;
  packet: AccountPacket;
  onJoin: (gameId: number, key: string) => void;
}) {
  const [tab, setTab] = useState<"games" | "keys">("games");
  return (
    <div className="flex flex-col gap-6 lg:min-h-0 lg:flex-1">
      <CreateGame client={client} onJoin={onJoin} />
      {/* Capped on a phone like the directory; filling its column on a wide screen. */}
      <section className={`flex max-h-[50dvh] min-h-64 flex-col lg:max-h-none lg:min-h-0 lg:flex-1 ${CARD}`}>
        <div className="flex shrink-0 gap-1 border-b border-edge px-3 pt-2 text-sm">
          <Tab active={tab === "games"} onClick={() => setTab("games")}>
            Your games ({packet.games.length})
          </Tab>
          <Tab active={tab === "keys"} onClick={() => setTab("keys")}>
            Saved keys ({packet.keys.length})
          </Tab>
        </div>
        {tab === "games" ? (
          <YourGames client={client} packet={packet} onJoin={onJoin} />
        ) : (
          <SavedKeys client={client} keys={packet.keys} onJoin={onJoin} />
        )}
      </section>
    </div>
  );
}

// The same tab look as the log in / sign up switch.
function Tab({ active, onClick, children }: { active: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      type="button"
      className={`flex-1 border-b-2 py-1.5 ${active ? "border-accent text-ink" : "border-transparent text-ink-dim hover:text-ink"}`}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

const CARD = "border border-edge bg-panel";
// A tab's list: fills the card and is the only thing in it that scrolls. Every row closes with its
// own bottom border, the last included, so a short list ends cleanly above the empty space.
const LIST = "min-h-0 flex-1 overflow-y-auto [&>li]:border-b [&>li]:border-edge";

function SavedKeys({
  client,
  keys,
  onJoin,
}: {
  client: Client;
  keys: AccountKey[];
  onJoin: (gameId: number, key: string) => void;
}) {
  const flash = useFlash();

  if (keys.length === 0) {
    return <p className="p-3 text-sm text-ink-dim">No saved keys. Joining a game while logged in saves its key here.</p>;
  }

  return (
    <>
      <ul className={LIST}>
        {keys.map((saved) => (
          <li key={`${saved.game_id}:${saved.key}`} className="flex flex-wrap items-center gap-2 px-3 py-2">
            <div className="flex w-full min-w-0 flex-col gap-0.5">
              <span className="text-sm text-ink">
                Game {saved.game_id} <span className="text-ink-dim">· {privilegesLabel(saved.privileges)}</span>
              </span>
              <span className="break-all font-mono text-xs text-ink-dim">{saved.key}</span>
            </div>
            <Button size="sm" onClick={() => onJoin(saved.game_id, saved.key)}>
              Join
            </Button>
            <Button size="sm" variant="ghost" onClick={() => void copyToClipboard(saved.key, flash, "Key copied.")}>
              Copy
            </Button>
            <ConfirmButton
              title="Forget key"
              body="The key is removed from this account. It still works; copy it first if you may want it back."
              confirm="Forget"
              variant="ghost"
              onConfirm={async () => void flash.reply(await client.forget_key(saved))}
            >
              Forget
            </ConfirmButton>
          </li>
        ))}
      </ul>
      {flash.message && (
        <div className="shrink-0 border-t border-edge px-3 py-2">
          <FlashLine flash={flash} />
        </div>
      )}
    </>
  );
}

function YourGames({
  client,
  packet,
  onJoin,
}: {
  client: Client;
  packet: AccountPacket;
  onJoin: (gameId: number, key: string) => void;
}) {
  const flash = useFlash();

  if (packet.games.length === 0) {
    return <p className="p-3 text-sm text-ink-dim">No games yet. Create one above.</p>;
  }

  return (
    <>
      <ul className={LIST}>
        {packet.games.map((id) => (
          <GameRow
            key={id}
            id={id}
            keys={packet.keys.filter((k) => k.game_id === id)}
            onJoin={onJoin}
            onEnd={async () => void flash.reply(await client.end_game(id))}
          />
        ))}
      </ul>
      {flash.message && (
        <div className="shrink-0 border-t border-edge px-3 py-2">
          <FlashLine flash={flash} />
        </div>
      )}
    </>
  );
}

// One of the account's games. Joining needs a key, and an account can hold several for one game
// (its admin key and player keys, say). With more than one the row asks which to join with,
// naming each by what it permits, rather than picking for you.
function GameRow({
  id,
  keys,
  onJoin,
  onEnd,
}: {
  id: number;
  keys: AccountKey[];
  onJoin: (gameId: number, key: string) => void;
  onEnd: () => Promise<void>;
}) {
  const [chosen, setChosen] = useState(keys[0]?.key ?? "");
  // A key forgotten elsewhere drops out of `keys`; fall back rather than join with a stale one.
  const key = keys.some((k) => k.key === chosen) ? chosen : (keys[0]?.key ?? "");
  return (
    <li className="flex flex-wrap items-center gap-2 px-3 py-2">
      <span className="min-w-0 flex-1 text-sm text-ink">
        Game {id}
        {keys.length === 1 && <span className="text-ink-dim"> · {privilegesLabel(keys[0].privileges)}</span>}
      </span>
      {keys.length > 1 && (
        <Select
          value={key}
          onChange={(e) => setChosen(e.target.value)}
          options={keys.map((k) => ({ value: k.key, label: privilegesLabel(k.privileges) }))}
          className="order-last w-full"
        />
      )}
      {keys.length > 0 && (
        <Button size="sm" onClick={() => onJoin(id, key)}>
          Join
        </Button>
      )}
      <ConfirmButton
        title="End game"
        body={`Game ${id} will stop running for everyone. This cannot be undone.`}
        confirm="End game"
        onConfirm={onEnd}
      >
        End game
      </ConfirmButton>
    </li>
  );
}

function CreateGame({
  client,
  onJoin,
}: {
  client: Client;
  onJoin: (gameId: number, key: string) => void;
}) {
  const [created, setCreated] = useState<GameCreationPacket | null>(null);
  const [busy, setBusy] = useState(false);
  const flash = useFlash();
  // Its own line, inside the modal: the section's line sits behind the backdrop.
  const copyFlash = useFlash();

  async function create() {
    setBusy(true);
    const reply = await client.create_game();
    setBusy(false);
    if (flash.reply(reply)) setCreated(reply.value);
  }

  return (
    <section className={`flex shrink-0 flex-col gap-2 p-3 ${CARD}`}>
      <Button disabled={busy} onClick={() => void create()}>
        Create game
      </Button>
      <FlashLine flash={flash} />
      <Modal
        open={created !== null}
        onClose={() => setCreated(null)}
        title="Game created"
        footer={
          <>
            <Button variant="ghost" onClick={() => created && void copyToClipboard(created.admin_key, copyFlash, "Key copied.")}>
              Copy key
            </Button>
            <Button
              onClick={() => {
                if (created) {
                  onJoin(created.game_id, created.admin_key);
                  setCreated(null);
                }
              }}
            >
              Join now
            </Button>
          </>
        }
      >
        {created && (
          <div className="flex flex-col gap-2 text-sm text-ink">
            <p>
              Game id: <span className="font-medium">{created.game_id}</span>
            </p>
            <p className="break-all font-mono">{created.admin_key}</p>
            <p className="text-ink-dim">
              This key is saved to your account. You can find it again under Saved keys.
            </p>
            <FlashLine flash={copyFlash} />
          </div>
        )}
      </Modal>
    </section>
  );
}
