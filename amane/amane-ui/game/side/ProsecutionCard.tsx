// One prosecution, sized for the top panel's horizontal row. Everything a prosecution needs to be
// driven (signal, advance, pick counsel, open its trial) lives here — the panel is just the strip
// that lays these out.
import { useState } from "react";
import type { Action, ActorDisplay } from "amane-client/bindings.ts";
import { slotKeyFromString, slotKeyToString } from "amane-client/bindings.ts";
import type { ProsecutionData } from "amane-client/game/types.ts";
import type { Side } from "amane-client/game/prosecution.ts";
import { awaitingHost, presentingSide, sideSignals } from "amane-client/game/prosecution.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { phaseLabel, playerLabel, refFromDisplay, refLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Select } from "../../kit/Input.tsx";
import { useGameUi, useView } from "../game_ui.ts";

export function ProsecutionCard({ id, data }: { id: string; data: ProsecutionData }) {
  const ui = useGameUi();
  const view = useView();
  const session = useSession();
  const flash = useFlash();
  const [lawyerChoice, setLawyerChoice] = useState("");

  const isAdmin = ui.viewer === "System";
  const isFrozen = view.frozen(data.viewport);

  function displayString(display: ActorDisplay): string {
    return refLabel(refFromDisplay(display, view.orgs), view.players);
  }

  // Which side of a prosecution the viewer is on. Read from what the engine told this view
  // privately, never inferred from the displays — an anonymous prosecutor reads Mysterious in their
  // own copy of the snapshot.
  function mySide(): Side | null {
    const side = view.own_prosecutions.get(id);
    if (side === undefined) return null;
    return side === "Prosecutor" ? "prosecution" : "defense";
  }

  // Whether the viewer is defence counsel — counsel has no side of their own, but ends the defence's
  // presentation exactly as the defendant does, and no view remembers it, so it is read off the
  // snapshot.
  function isLawyer(): boolean {
    if (isAdmin) return false;
    const display = data.lawyer_display;
    return display !== null && typeof display !== "string" && "Raw" in display && slotKeyToString(display.Raw) === ui.viewer;
  }

  const side = mySide();
  const floor = presentingSide(data.phase);
  const holdsFloor = floor !== null && (floor === "prosecution" ? side === "prosecution" : side === "defense" || isLawyer());

  // The signal the viewer still has to give, if this phase takes one from them.
  let mySignal: { verb: string } | null = null;
  if (holdsFloor) {
    mySignal = isFrozen ? null : { verb: "end my turn" };
  } else if (side !== null && !awaitingHost(data.phase) && !isFrozen) {
    const mine = sideSignals(data.phase).find((s) => s.side === side);
    mySignal = mine && !mine.done ? { verb: mine.verb } : null;
  }

  // Counsel is the defendant's to choose, once, while they are still in custody.
  const canPickLawyer =
    side === "defense" && data.lawyer_display === null && typeof data.phase !== "string" && "Custody" in data.phase && !isFrozen;
  const lawyerCandidates = [...view.players.keys()].filter((pid) => pid !== ui.viewer);

  function openChannel() {
    if (data.trial_channel) ui.select({ kind: "channel", id: data.trial_channel });
  }

  async function run(payload: Action, ok: string) {
    const reply = await session.submit_action({ actor: viewActor(ui.viewer), timestamp: Date.now(), payload });
    flash.reply(reply, ok);
  }
  const advance = () => run({ AdvanceProsecution: { prosecution_id: slotKeyFromString(id) } }, "Advanced.");
  const terminate = () => run({ TerminateProsecution: { prosecution_id: slotKeyFromString(id), verdict: null } }, "Terminated.");
  const sendSignal = () => run({ SignalReady: { prosecution_id: slotKeyFromString(id) } }, "Signalled.");
  function pickLawyer() {
    if (!lawyerChoice) return;
    run({ SelectLawyer: { prosecution_id: slotKeyFromString(id), lawyer_id: slotKeyFromString(lawyerChoice) } }, "Counsel selected.");
  }

  return (
    <div className="flex w-[min(18rem,85vw)] shrink-0 flex-col gap-1.5 border border-edge bg-panel p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium text-ink">
          {displayString(data.prosecutor_display)} <span className="text-ink-dim">vs</span> {displayString(data.defendant_display)}
        </span>
        {isFrozen && (
          <span
            className="px-1.5 text-xs uppercase tracking-wide"
            style={{ color: "var(--color-event-alarm)" }}
            title="You lost presence — showing the last state you received."
          >
            frozen
          </span>
        )}
      </div>

      <span className="text-xs text-ink-dim">{phaseLabel(data.phase)}</span>

      {data.lawyer_display && <span className="text-xs text-ink-dim">defended by {displayString(data.lawyer_display)}</span>}

      {sideSignals(data.phase).map((signal) => (
        <span key={signal.side} className={`text-xs ${signal.done ? "text-ok-text" : "text-ink-dim"}`}>
          {signal.done ? "✓" : "○"} {signal.side} {signal.done ? signal.verb : `not ${signal.verb}`}
        </span>
      ))}

      {canPickLawyer && (
        <div className="flex items-center gap-1">
          <Select
            value={lawyerChoice}
            onChange={(e) => setLawyerChoice(e.target.value)}
            options={[
              { value: "", label: "choose counsel…" },
              ...lawyerCandidates.map((pid) => ({ value: pid, label: playerLabel(pid, view.players) })),
            ]}
            className="min-w-0 flex-1"
          />
          <Button variant="ghost" size="sm" disabled={!lawyerChoice} onClick={pickLawyer}>
            retain
          </Button>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-1">
        {data.trial_channel && (
          <Button variant="ghost" size="sm" onClick={openChannel}>
            open trial
          </Button>
        )}

        {mySignal && (
          <Button variant="default" size="sm" onClick={sendSignal}>
            {mySignal.verb}
          </Button>
        )}

        {isAdmin && (
          <>
            <Button
              variant="ghost"
              size="sm"
              style={awaitingHost(data.phase) ? { color: "var(--color-event-alarm)" } : undefined}
              onClick={advance}
            >
              {awaitingHost(data.phase) ? "approve" : "advance"}
            </Button>
            <Button variant="danger" size="sm" onClick={terminate}>
              terminate
            </Button>
          </>
        )}
      </div>

      <FlashLine flash={flash} />
    </div>
  );
}
