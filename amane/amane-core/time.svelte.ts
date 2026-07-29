// The one place deciding what goes in an action's timestamp field, so nothing calls Date.now()
// directly.
//
// The value does not survive: yagami overwrites it on arrival, since game time is the server's and
// a client whose clock lies or is simply wrong must not be able to move the engine's or backdate
// an action. What this fills is a required wire field, not the time the action happens at.
export function now(): number {
  return Date.now();
}
