// Prosecution phase logic shared by handlers and surfaces: every question asked of a phase is
// answered here, once. No text lives here — see text.ts for phaseLabel/phaseAnnouncementKey.
import type { ProsecutionPhaseView } from "../bindings.ts";

// Whether two snapshots are the same PHASE, ignoring the ready/done flags inside it. Mirrors
// ProsecutionPhaseView::same_phase in the engine.
//
// Signalling ready changes the snapshot without changing the phase, so comparing whole values
// would announce the prosecution again on every signal. The subphase IS compared — grace →
// presentation is a real transition.
export function phaseViewEqual(a: ProsecutionPhaseView, b: ProsecutionPhaseView): boolean {
  if (a === "Voting" || b === "Voting") return a === b;
  if ("Custody" in a || "Custody" in b) return "Custody" in a && "Custody" in b;

  const [x, y] = [a.Trial, b.Trial];
  if ("Debate" in x || "Debate" in y) return "Debate" in x && "Debate" in y;
  if ("Prosecutor" in x && "Prosecutor" in y) return x.Prosecutor === y.Prosecutor;
  if ("Defense" in x && "Defense" in y) return x.Defense === y.Defense;
  return false;
}

export type Side = "prosecution" | "defense";

// The side presenting right now: the one side with the floor during its Presentation subphase.
// Null in every other phase, including a side's Grace, when nobody has started.
export function presentingSide(phase: ProsecutionPhaseView): Side | null {
  if (phase === "Voting" || "Custody" in phase) return null;
  const trial = phase.Trial;
  if ("Prosecutor" in trial) return trial.Prosecutor === "Presentation" ? "prosecution" : null;
  if ("Defense" in trial) return trial.Defense === "Presentation" ? "defense" : null;
  return null;
}

// Each side's signal, in the phases that wait on both: custody asks whether each is ready, debate
// whether each is done. Empty in phases with no signal.
export function sideSignals(phase: ProsecutionPhaseView): { side: Side; done: boolean; verb: "ready" | "done" }[] {
  if (phase === "Voting") return [];
  if ("Custody" in phase) {
    return [
      { side: "prosecution", done: phase.Custody.prosecutor_ready, verb: "ready" },
      { side: "defense", done: phase.Custody.defense_ready, verb: "ready" },
    ];
  }
  if ("Debate" in phase.Trial) {
    return [
      { side: "prosecution", done: phase.Trial.Debate.prosecutor_done, verb: "done" },
      { side: "defense", done: phase.Trial.Debate.defense_done, verb: "done" },
    ];
  }
  return [];
}

// Whether a non-autonomous prosecution has met the condition to leave this phase and is waiting on
// a host. Only the two phases that can be held carry the flag.
export function awaitingHost(phase: ProsecutionPhaseView): boolean {
  if (phase === "Voting") return false;
  if ("Custody" in phase) return phase.Custody.awaiting_host;
  return "Debate" in phase.Trial && phase.Trial.Debate.awaiting_host;
}
