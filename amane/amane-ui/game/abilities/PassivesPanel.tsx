// Read-only list of the current viewer's passives, opened from a button beside Abilities.
// Passives aren't used (no charges), just observed — some carry data (e.g. a vote amplification
// multiplier), which is shown inline.
import { useState } from "react";
import type { PassiveType } from "amane-client/bindings.ts";
import { passiveDescription } from "amane-client/text.ts";
import { Button } from "../../kit/Button.tsx";
import { Modal } from "../../kit/Modal.tsx";
import { useView } from "../game_ui.ts";

function prettyPassive(p: PassiveType): string {
  // string variants: split camelCase ("CustodyBugReceiver" -> "Custody Bug Receiver")
  if (typeof p === "string") return p.replace(/([a-z])([A-Z])/g, "$1 $2");
  if ("VoteAmplification" in p) return `Vote Amplification (×${p.VoteAmplification.multiplier})`;
  if ("ContactLogs" in p) return `Contact Logs (${p.ContactLogs})`;
  return "Passive";
}

export function PassivesPanel() {
  const view = useView();
  const [open, setOpen] = useState(false);
  // Which passives have their description expanded, keyed by passive id.
  const [shown, setShown] = useState<Record<string, boolean>>({});

  const passives = [...view.passives.entries()];

  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        Passives
      </Button>

      <Modal open={open} onClose={() => setOpen(false)} title="Passives">
        <div className="flex flex-col gap-1.5">
          {passives.map(([id, pv]) => {
            const desc = passiveDescription(pv.type);
            return (
              <div key={id} className="border border-edge bg-raised">
                <div className="flex items-center justify-between gap-2 px-3 py-2 text-sm text-ink">
                  <span>{prettyPassive(pv.type)}</span>
                  {desc && (
                    <Button
                      variant={shown[id] ? "default" : "ghost"}
                      size="sm"
                      className="aspect-square shrink-0 px-0"
                      aria-label="Toggle description"
                      aria-pressed={!!shown[id]}
                      title="What this does"
                      onClick={() => setShown((prev) => ({ ...prev, [id]: !prev[id] }))}
                    >
                      ?
                    </Button>
                  )}
                </div>
                {shown[id] && desc && <p className="border-t border-edge px-3 py-2 text-sm text-ink-dim">{desc}</p>}
              </div>
            );
          })}
          {passives.length === 0 && <p className="py-2 text-sm text-ink-dim">No passives.</p>}
        </div>
      </Modal>
    </>
  );
}
