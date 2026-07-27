// The client clock — one place deciding what a client puts in an action's timestamp field, so
// nothing calls Date.now() directly.
//
// The value does not survive: yagami overwrites it on arrival (game time is the server's, so a
// client that lies or whose clock is simply wrong cannot move the engine's clock or backdate an
// action). What this fills is a required wire field, not the time the action happens at.
//
// That is also why the settable offset is gone. It shifted the whole client's sense of "now" to
// time-travel an engine hosted IN-PROCESS, which is a thing only armonia ever was; against a server
// it was overwritten and did nothing. The emitted high-water mark went with it — it existed solely
// to stop an offset rewinding below a timestamp already sent.
export function now(): number {
  return Date.now();
}
