// The host's action timeline: every request a connection submitted and how it came out. Lives on
// the System view, which is the only one that sees Admin-gated output.
import { useState } from "react";
import { Virtuoso } from "react-virtuoso";
import type { Action, ActionActor } from "amane-client/bindings.ts";
import { slotKeyToString } from "amane-client/bindings.ts";
import { playerLabel } from "amane-client/text.ts";
import type { ActionLogEntry } from "amane-client/game/types.ts";
import type { View } from "amane-client/game/view.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { Modal } from "../../kit/Modal.tsx";

// ---- formatting (deliberately local: this is host tooling, not shared game text) ----

function actionName(a: Action): string {
  return Object.keys(a)[0];
}

function prettyArg(k: string): string {
  const s = k.replace(/_id$/, "").replace(/_/g, " ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function isSlotKey(v: unknown): v is { idx: number; version: number } {
  return !!v && typeof v === "object" && typeof (v as { idx: unknown }).idx === "number" && typeof (v as { version: unknown }).version === "number";
}

// Action payload fields whose slot key is a PLAYER, resolving to a name in view.players. Every
// other slot key (channel_id, bug_id, lounge_id, notebook_id, poll_id, prosecution_id, a bare `id`,
// ...) is a different object and must NOT be shown as a player name.
const PLAYER_KEY_FIELDS = new Set([
  "target",
  "target_id",
  "actor_id",
  "player_id",
  "creator_id",
  "contactor_id",
  "contacted_id",
  "performer",
  "sacrifice",
  "name_target",
  "kidnapper",
  "victim_id",
  "accuser_id",
  "user",
  "prosecutor",
  "defendant",
]);

function fmtArg(key: string, v: unknown, players: View["players"]): string {
  if (v === null || v === undefined) return "—";
  if (isSlotKey(v)) {
    const k = slotKeyToString(v);
    return PLAYER_KEY_FIELDS.has(key) ? playerLabel(k, players) : k;
  }
  if (typeof v === "string") return v;
  if (typeof v === "boolean") return String(v);
  if (typeof v === "number") return isFinite(v) ? String(v) : `${v}`;
  if (Array.isArray(v)) {
    const s = v.map((x) => fmtArg(key, x, players));
    return s.length > 3 ? `${s.slice(0, 3).join(", ")} +${s.length - 3}` : s.join(", ");
  }
  if (typeof v === "object") {
    const obj = v as Record<string, unknown>;
    if ("Some" in obj) return fmtArg(key, obj.Some, players);
    if ("None" in obj) return "—";
    const entries = Object.entries(obj);
    if (entries.length === 0) return "∅";
    return entries.map(([k, x]) => `${prettyArg(k)}: ${fmtArg(k, x, players)}`).join(", ");
  }
  return String(v);
}

function payloadArgs(a: Action, players: View["players"]): { key: string; value: string }[] {
  const name = Object.keys(a)[0];
  const data = (a as unknown as Record<string, unknown>)[name];
  if (!data || typeof data !== "object") return [];
  return Object.entries(data as Record<string, unknown>).map(([k, v]) => ({ key: prettyArg(k), value: fmtArg(k, v, players) }));
}

function actorText(actor: ActionActor, players: View["players"]): string {
  if (actor === "Admin") return "Admin";
  if (actor === "System") return "System";
  if ("Player" in actor) return playerLabel(slotKeyToString(actor.Player), players);
  return `Org ${slotKeyToString(actor.Organization.org_id)}`;
}

// Game time in milliseconds since the sandbox's zero, rendered as HH:MM:SS.
function timeText(ms: number): string {
  const s = Math.floor(ms / 1000) % 60;
  const m = Math.floor(ms / 60000) % 60;
  const h = Math.floor(ms / 3600000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(h)}:${pad(m)}:${pad(s)}`;
}

function Row({ entry, players }: { entry: ActionLogEntry; players: View["players"] }) {
  return (
    <div className="flex flex-col gap-1 border-b border-edge py-1.5 text-sm sm:flex-row sm:items-start sm:gap-3">
      <span className="shrink-0 pt-0.5 font-mono text-sm tabular-nums text-ink-dim sm:w-16">{timeText(entry.time)}</span>
      <span className="min-w-0 shrink-0 truncate pt-0.5 text-ink-dim sm:w-40">{actorText(entry.action.actor, players)}</span>
      <div className="min-w-0 flex-1">
        <div className="font-medium text-ink">{actionName(entry.action.payload)}</div>
        {payloadArgs(entry.action.payload, players).map((r) => (
          <div key={r.key} className="text-xs text-ink-dim">
            <span>{r.key}:</span> {r.value}
          </div>
        ))}
      </div>
    </div>
  );
}

export function TimelineViewer() {
  const session = useSession();
  const view = session.game.system_view();
  const entries = view.action_log;

  const [open, setOpen] = useState(false);
  // Which action variants to leave OUT of the timeline. A hidden variant is skipped forever, even
  // once the log contains it again — unchecking restores it.
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [filterOpen, setFilterOpen] = useState(false);

  function toggle(variant: string) {
    setHidden((prev) => {
      const next = new Set(prev);
      if (next.has(variant)) next.delete(variant);
      else next.add(variant);
      return next;
    });
  }

  // Every distinct action variant this view has seen, so the filter never offers a name that has
  // not appeared.
  const variants = [...new Set(entries.map((e) => actionName(e.action.payload)))].sort();
  const shown = entries.filter((e) => !hidden.has(actionName(e.action.payload)));

  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        Timeline
      </Button>

      <Modal open={open} onClose={() => setOpen(false)} title="Action Timeline" width="42rem" scroll="content">
        {/* A definite height, not h-full: Virtuoso has no height of its own (it renders only the rows
            that fit its box), so inside a content-sized modal the list would collapse to nothing. */}
        <div className="flex h-[70dvh] min-h-0 flex-col gap-2">
          <p className="shrink-0 text-sm text-ink-dim">
            Every action requested by a connection, and how it came out — newest at the bottom, like a chat.
            Server-initiated work (ticks, time skips) is not shown.
          </p>

          <div className="flex shrink-0 items-center gap-2">
            <Button variant="ghost" size="sm" onClick={() => setFilterOpen(!filterOpen)}>
              {filterOpen ? "Close filter" : `Filter${hidden.size ? ` (${hidden.size})` : ""}`}
            </Button>
            {hidden.size > 0 && (
              <Button variant="ghost" size="sm" onClick={() => setHidden(new Set())}>
                Show all
              </Button>
            )}
          </div>

          {filterOpen && (
            <div className="grid max-h-56 shrink-0 grid-cols-2 gap-x-3 gap-y-1.5 overflow-y-auto border border-edge p-2">
              {variants.length === 0 ? (
                <p className="text-sm text-ink-dim">No variants yet.</p>
              ) : (
                variants.map((variant) => (
                  <label key={variant} className="flex items-center gap-2 text-sm text-ink">
                    <input type="checkbox" checked={!hidden.has(variant)} onChange={() => toggle(variant)} />
                    <span className="truncate">{variant}</span>
                  </label>
                ))
              )}
            </div>
          )}

          {shown.length === 0 ? (
            <p className="text-sm text-ink-dim">No actions to show.</p>
          ) : (
            <div className="min-h-0 flex-1">
              <Virtuoso
                style={{ height: "100%" }}
                data={shown}
                followOutput="smooth"
                itemContent={(_, entry) => <Row entry={entry} players={view.players} />}
              />
            </div>
          )}
        </div>
      </Modal>
    </>
  );
}
