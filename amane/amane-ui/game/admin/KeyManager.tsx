// Minting the keys that let anyone else into the game, and managing the ones that exist.
//
// Creating a game mints exactly one key — the admin's — so every other participant's credential has
// to come from here. A key is not a person and not a connection: it is a privilege set, and
// "associating it with an actor" means naming the slots its holder may play as.
//
// The set that already exists arrives on the KeyRoster, a whole-set server command gated to admins,
// and is rendered below as one list: what each key permits, and how to change or revoke it.
// Authority to touch a given key is the server's to decide — a supervisor key, or the full set, or
// one's own key — and a refused control surfaces its error here, so the table offers everything and
// the server denies what it must. A freshly created key lands in the same list the moment the
// roster refreshes, no separate handling. Secret tokens are shown plainly: keys are how someone gets
// in, handed over out of band (hence each row's Copy), not worth treating as the crown jewels.
import { useState } from "react";
import type { ActorScope, Capability, PrivilegeSet } from "amane-client/bindings.ts";
import { slotKeyFromString, slotKeyToString } from "amane-client/bindings.ts";
import { playerLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { copyToClipboard } from "../../kit/clipboard.ts";
import { ConfirmButton } from "../../kit/ConfirmButton.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Select } from "../../kit/Input.tsx";
import { Modal } from "../../kit/Modal.tsx";

// What a key may do beyond acting as its actors. Supervise never appears without Administer
// because a key holding it alone is inert — every control is gated on Administer first, and
// Supervise only widens which OTHER keys an administrator may touch.
type Grant = "player" | "admin" | "supervisor";

const GRANTS: { value: Grant; label: string }[] = [
  { value: "player", label: "Player — no administration" },
  { value: "admin", label: "Administrator — manages ordinary keys" },
  { value: "supervisor", label: "Supervisor — manages administrators too" },
];

const CAPABILITIES: Record<Grant, Capability[]> = {
  player: [],
  admin: ["Administer"],
  supervisor: ["Administer", "Supervise"],
};

// The capabilities a set amounts to, pinned to the three grant bins the UI offers.
function grantFor(caps: Capability[]): Grant {
  if (caps.includes("Supervise")) return "supervisor";
  if (caps.includes("Administer")) return "admin";
  return "player";
}

function grantLabel(privileges: PrivilegeSet): string {
  return GRANTS.find((g) => g.value === grantFor(privileges.capabilities))!.label;
}

function scopeLabel(privileges: PrivilegeSet, players: ReadonlyMap<string, { display_name: string | null }>): string {
  if (privileges.actors === "All") return "Every actor";
  return (
    privileges.actors.Only.map((a) => playerLabel(slotKeyToString(a), players))
      .filter(Boolean)
      .join(", ") || "No actors"
  );
}

export function KeyManager() {
  const session = useSession();
  const sys = session.game.system_view();
  const [open, setOpen] = useState(false);
  const flash = useFlash();

  // ---- minting ----
  const [grant, setGrant] = useState<Grant>("player");
  const [everyActor, setEveryActor] = useState(false);
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);

  const players = [...sys.players.keys()]
    .map((id) => ({ id, label: playerLabel(id, sys.players) }))
    .sort((a, b) => a.label.localeCompare(b.label));

  const keys = [...sys.keys.entries()].sort((a, b) => a[0].localeCompare(b[0]));

  function toggle(id: string) {
    setChosen((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function resetMint() {
    setChosen(new Set());
    setEveryActor(false);
    setGrant("player");
  }

  async function create() {
    // A key with neither actors nor administration can do nothing at all, and would only ever be
    // discovered as such by whoever was handed it.
    if (!everyActor && chosen.size === 0 && grant === "player") {
      flash.error("Pick at least one player, or grant administration.");
      return;
    }
    const actors: ActorScope = everyActor ? "All" : { Only: [...chosen].map(slotKeyFromString) };
    setBusy(true);
    const reply = await session.submit_control({
      Sim: { time: 0, data: { CreateKey: { actors, capabilities: CAPABILITIES[grant] } } },
    });
    setBusy(false);
    if (!flash.reply(reply)) return;
    // Every other control answers with a bare tag, so a response without the key means the reply
    // stream is not what this client thinks it is.
    if (typeof reply.value === "string" || !("KeyCreated" in reply.value)) {
      flash.error("The server answered something other than a key.");
      return;
    }
    // The new key lands in the ledger (and thus the list below) on the roster refresh that follows
    // the mint; nothing special is done with the key from the reply alone.
    resetMint();
    flash.ok("Key created — it's in the list below.");
  }

  // ---- editing an existing key ----
  const [editing, setEditing] = useState<string | null>(null);
  const [editGrant, setEditGrant] = useState<Grant>("player");
  const [editEvery, setEditEvery] = useState(false);
  const [editChosen, setEditChosen] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState(false);

  function beginEdit(key: string) {
    const privileges = sys.keys.get(key);
    if (!privileges) return;
    setEditing(key);
    setEditGrant(grantFor(privileges.capabilities));
    setEditEvery(privileges.actors === "All");
    setEditChosen(privileges.actors === "All" ? new Set() : new Set(privileges.actors.Only.map(slotKeyToString)));
  }

  function toggleEdit(id: string) {
    setEditChosen((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function save() {
    const key = editing;
    if (!key) return;
    setSaving(true);
    const actors: ActorScope = editEvery ? "All" : { Only: [...editChosen].map(slotKeyFromString) };
    const setCaps = await session.submit_control({
      Sim: { time: 0, data: { SetCapabilities: { key, capabilities: CAPABILITIES[editGrant] } } },
    });
    if (!flash.reply(setCaps)) {
      setSaving(false);
      return;
    }
    const setScope = await session.submit_control({ Sim: { time: 0, data: { SetActorScope: { key, actors } } } });
    setSaving(false);
    if (!flash.reply(setScope)) return;
    flash.ok("Key updated.");
    setEditing(null);
  }

  async function revoke(key: string) {
    const reply = await session.submit_control({ Sim: { time: 0, data: { RevokeKey: { key } } } });
    if (flash.reply(reply, "Key revoked.") && editing === key) setEditing(null);
  }

  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        Keys
      </Button>

      {/* The form for a new key stays put; only the list of existing keys scrolls. */}
      <Modal open={open} onClose={() => setOpen(false)} title="Keys" width="42rem" scroll="content">
        <div className="flex min-h-0 flex-1 flex-col gap-3">
          {/* mint */}
          <div className="flex shrink-0 flex-col gap-3">
            <p className="text-sm text-ink-dim">
              A key is how someone gets in. Choose the players its holder may act as, hand it over out of band,
              and they join with it. Every existing key shows below with what it may do.
            </p>

            <label className="flex items-center gap-2 text-sm text-ink">
              <input type="checkbox" checked={everyActor} onChange={(e) => setEveryActor(e.target.checked)} />
              Every actor, including ones added later
            </label>

            {/* Capped by the screen, not a fixed height, so the form never crowds out the list. */}
            <div className={`max-h-[20dvh] overflow-y-auto border border-edge ${everyActor ? "opacity-50" : ""}`}>
              {players.length === 0 ? (
                <p className="px-2 py-1 text-sm text-ink-dim">No players yet — add some first.</p>
              ) : (
                players.map((player) => (
                  <label key={player.id} className="flex items-center gap-2 px-2 py-1 text-sm text-ink hover:bg-raised">
                    <input type="checkbox" disabled={everyActor} checked={chosen.has(player.id)} onChange={() => toggle(player.id)} />
                    {player.label}
                  </label>
                ))
              )}
            </div>

            <Select value={grant} onChange={(e) => setGrant(e.target.value as Grant)} options={GRANTS} />

            <Button disabled={busy} onClick={create}>
              Create key
            </Button>

            <FlashLine flash={flash} />
          </div>

          {/* the ledger: the same list a fresh key lands in, and the only part that scrolls */}
          <div className="flex min-h-0 flex-1 flex-col gap-2 border-t border-edge pt-3">
            <p className="shrink-0 text-sm text-ink-dim">Keys, as the server holds them.</p>
            {keys.length === 0 && <p className="text-sm text-ink-dim">No keys yet — create one above.</p>}
            <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
              {keys.map(([key, privileges]) => (
                <div key={key} className="border border-edge p-2">
                  {editing === key ? (
                    <div className="flex flex-col gap-2">
                      <label className="flex items-center gap-2 text-sm text-ink">
                        <input type="checkbox" checked={editEvery} onChange={(e) => setEditEvery(e.target.checked)} />
                        Every actor, including ones added later
                      </label>
                      <div className={`max-h-32 overflow-y-auto border border-edge ${editEvery ? "opacity-50" : ""}`}>
                        {players.map((player) => (
                          <label key={player.id} className="flex items-center gap-1 px-2 py-1 text-sm text-ink hover:bg-raised">
                            <input
                              type="checkbox"
                              disabled={editEvery}
                              checked={editChosen.has(player.id)}
                              onChange={() => toggleEdit(player.id)}
                            />
                            {player.label}
                          </label>
                        ))}
                      </div>
                      <Select value={editGrant} onChange={(e) => setEditGrant(e.target.value as Grant)} options={GRANTS} />
                      <div className="flex gap-2">
                        <Button disabled={saving} onClick={save}>
                          Save
                        </Button>
                        <Button variant="ghost" onClick={() => setEditing(null)}>
                          Cancel
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <div className="flex items-center justify-between gap-2">
                      <div className="min-w-0">
                        <code className="block truncate font-mono text-xs">{key}</code>
                        <p className="text-xs text-ink-dim">{grantLabel(privileges)}</p>
                        <p className="text-xs text-ink-dim">{scopeLabel(privileges, sys.players)}</p>
                      </div>
                      <div className="flex shrink-0 gap-1.5">
                        <Button variant="ghost" size="sm" onClick={() => void copyToClipboard(key, flash)}>
                          Copy
                        </Button>
                        <Button variant="ghost" size="sm" onClick={() => beginEdit(key)}>
                          Edit
                        </Button>
                        <ConfirmButton
                          title="Revoke key"
                          body="Its holder is disconnected immediately, and the key stops working for good."
                          confirm="Revoke"
                          onConfirm={() => revoke(key)}
                        >
                          Revoke
                        </ConfirmButton>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      </Modal>
    </>
  );
}
