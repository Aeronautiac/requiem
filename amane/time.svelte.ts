// Central client clock. Every action timestamp flows through now() so a single offset
// can shift the whole client's sense of "now" — the time-travel knob. The offset
// accumulates (add_offset) and is applied on top of real wall-clock time. Anything
// that needs the current time for an action MUST use now(), never Date.now() directly,
// or it will ignore time travel.
let offset = $state(0);
// High-water mark of timestamps we've emitted. The engine enforces monotonic time
// progression, so we must never produce one below this.
let emitted = 0;

// Current client time: real time plus the accumulated time-travel offset. Records the
// high-water mark so offsets can't later rewind past a timestamp we've already sent.
export function now(): number {
  const t = Date.now() + offset;
  if (t > emitted) emitted = t;
  return t;
}

// Shift the client clock by delta (same units as timestamps). Accumulates. Rejected
// (returns false, applies nothing) if it would pull the clock below the last timestamp
// we've emitted — that would violate the engine's enforced time progression. A negative
// delta that still stays at or above the high-water mark (e.g. trimming a future offset)
// is allowed.
export function add_offset(delta: number): boolean {
  if (Date.now() + offset + delta < emitted) return false;
  offset += delta;
  return true;
}

// Reactive read of the accumulated offset, for display.
export function time_offset(): number {
  return offset;
}
