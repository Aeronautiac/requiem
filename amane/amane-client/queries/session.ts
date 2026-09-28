// What this connection's key permits. UX only: the server checks every action and control against
// its own ledger whatever a surface offers.
import type { ActionActor, ActorKey } from "../bindings.ts";
import { slotKeyFromString } from "../bindings.ts";
import type { Session } from "../session.ts";

// Whether admin surfaces are offered at all.
export function administers(session: Session): boolean {
  return session.privileges?.capabilities.includes("Administer") ?? false;
}

// Authority over OTHER administrators' keys.
export function supervises(session: Session): boolean {
  return session.privileges?.capabilities.includes("Supervise") ?? false;
}

// The actors this key names individually. Empty for `All`, which is not expanded: it covers actors
// created later, which no list can.
export function scopedActors(session: Session): ActorKey[] {
  const scope = session.privileges?.actors;
  return scope !== undefined && scope !== "All" ? scope.Only : [];
}

// Every view this connection may look through, System first. Read off the views that have received
// something rather than off the key's scope: an actor is only worth looking through once something
// has been delivered to it, and `All` names nobody. Empty until the first batch lands.
export function viewers(session: Session): string[] {
  const actors = [...session.game.views.keys()]
    .filter((key) => key !== "System")
    .sort((a, b) => parseInt(a) - parseInt(b));
  return administers(session) ? ["System", ...actors] : actors;
}

// The actor a view acts as: an actor's own key, or Admin for the System view.
export function viewActor(view_key: string): ActionActor {
  return view_key === "System" ? "Admin" : { Player: slotKeyFromString(view_key) };
}
