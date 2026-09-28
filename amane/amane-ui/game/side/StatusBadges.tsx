// Always visible rather than behind a button: being in custody or under the radar changes what you
// should be doing right now, so it is status, not a list.
//
// Own states only, which is the whole shape of ActorState — what you know about anyone ELSE comes
// from the event that announced it, never from here. Compact and wrapping: it sits in the bottom bar.
import { StateFlag } from "amane-client/bindings.ts";
import { statusAccent, tint } from "../../style.ts";
import { useView } from "../game_ui.ts";

// Ordered worst-first, so the badge that matters most reads first. Coloured off the closest public
// status accent it corresponds to; "Off the Record" has no public counterpart, so it takes the
// neutral default.
const BADGES: { flag: number; label: string; accent: string }[] = [
  { flag: StateFlag.Dead, label: "Dead", accent: statusAccent("dead") },
  { flag: StateFlag.Custody, label: "In Custody", accent: statusAccent("custody") },
  { flag: StateFlag.Kidnapped, label: "Kidnapped", accent: statusAccent("kidnapped") },
  { flag: StateFlag.Incarcerated, label: "Incarcerated", accent: statusAccent("incarcerated") },
  { flag: StateFlag.UnderTheRadar, label: "Off the Record", accent: statusAccent("default") },
  { flag: StateFlag.Ipp, label: "IPP", accent: statusAccent("ipp") },
];

export function StatusBadges() {
  const view = useView();
  const active = BADGES.filter((b) => (view.states & b.flag) !== 0);
  if (active.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {active.map((badge) => (
        <span key={badge.label} className="px-2 py-0.5 text-xs font-medium" style={tint(badge.accent)}>
          {badge.label}
        </span>
      ))}
    </div>
  );
}
