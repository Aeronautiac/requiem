// Names, chips and timestamps: the pieces every row is built from, so a name reads the same
// wherever it appears.
import type { View } from "amane-client/game/view.ts";
import type { Ref } from "amane-client/text.ts";
import { formatTime, refLabel } from "amane-client/text.ts";
import { refColor, tint } from "../style.ts";
import { useGameUi } from "./game_ui.ts";

// The pill look, shared with the composer, which has to build its chips as raw DOM (they sit
// inside a contenteditable) and must look exactly like these.
export const CHIP_CLASS = "box-decoration-clone px-0.5 py-0.5 font-medium";

// A coloured inline pill.
export function Chip({ label, colorVar }: { label: string; colorVar: string }) {
  return (
    <span className={CHIP_CLASS} style={tint(colorVar)}>
      {label}
    </span>
  );
}

// Anything that has a name: a player, a role, an org, System, an anonymous party. The one way a
// name renders, whether it came from an actor key, an engine display or a mention (convert with
// `refFromKey`, `refFromDisplay`, or pass a mention as it is).
//
// `chip` is the pill form, for a name that is a prominent reference (an announcement, a mention,
// a message sender); plain coloured text otherwise. A player the view knows opens the profile
// menu, unless `menu` is off (a name inside its own clickable row).
export function Entity({
  of,
  view,
  chip = false,
  menu = true,
}: {
  of: Ref;
  view: View;
  chip?: boolean;
  menu?: boolean;
}) {
  const ui = useGameUi();
  const label = refLabel(of, view.players);
  const color = refColor(of, view);
  const body = chip ? (
    <Chip label={label} colorVar={color} />
  ) : (
    <span className="font-medium" style={{ color }}>
      {label}
    </span>
  );
  if (!menu || of.kind !== "player" || !view.players.has(of.id)) return body;
  const id = of.id;
  return (
    <button
      type="button"
      className="cursor-pointer text-left align-baseline hover:opacity-80"
      onClick={() => ui.openPlayerMenu(id)}
    >
      {body}
    </button>
  );
}

// A game moment: raw elapsed game time, never a wall-clock date (time travel untethers the two).
export function TimeStamp({ timestamp }: { timestamp: number }) {
  return (
    <span className="shrink-0 border border-edge bg-panel px-1.5 py-px text-xs tabular-nums text-ink-dim">
      {formatTime(timestamp)}
    </span>
  );
}
