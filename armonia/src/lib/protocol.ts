// ////////////////////////////////////////////////////////////
// STREAMING PROTOCOL SEAM
// ////////////////////////////////////////////////////////////
//
// Target shape for when the server ticks on its own and pushes updates to every
// client. It is NOT wired into the live Tauri transport yet — this file locks the
// protocol shape so the server can be built against it.
//
// Two things a client receives, split by ENVELOPE (see RequestReply / CommandBatch):
//
//   - Full responses are ONLY ever replies to this client's own requests.
//   - External actions (server ticks, other players' actions) deliver COMMANDS only.
//
// Ordering is a SEPARATE axis from that split, unified by `seq`: every command batch
// this client will ever apply carries one, whether it arrived as a reply or a push.
// The client applies batches strictly in `seq` order via the Sequencer, independent
// of which path delivered them or in what order they physically arrived. This is the
// whole defence against the client-side reorder desync: reply and push have no
// defined arrival order relative to each other, so arrival order must not be trusted.

import type { ActionRequest, AppExecution, CommandPayload } from "../bindings";

// Server-stamped, per-client, strictly increasing (by 1). The first batch is seq 1;
// seq 0 means "nothing applied yet". It is the single source of truth for the order
// in which this client must apply command batches.
export type Seq = number;

// The unit of client state change: commands to apply atomically and in seq order.
// Commands are already recipient-filtered by the server for this client.
export interface CommandBatch {
  seq: Seq;
  commands: CommandPayload[];
}

// The reply to THIS client's own request: the full execution result (so the caller
// can read response data / errors synchronously) plus the seq that orders the state
// effect carried inside it. The caller reads `execution` immediately for data/errors,
// but must route its effect through the Sequencer like any other unit — never apply it
// straight from the reply.
export interface RequestReply {
  seq: Seq;
  execution: AppExecution;
}

// The router seam once the push channel exists. `sendAction` still returns a full,
// now seq-tagged, response. `onCommands` is the push stream for external actions.
export interface StreamingRouter {
  sendAction(action: ActionRequest): Promise<RequestReply>;
  // Subscribe to command batches caused by external actions (ticks / other clients).
  // Returns an unsubscribe function.
  onCommands(handler: (batch: CommandBatch) => void): () => void;
  quit(): void;
}

// One seq-ordered unit of client state change. `run` applies its effect: for a pushed
// batch that's "apply these commands"; for a reply it's "handle the response data,
// then apply the commands" — bundled so a reply lands as a single ordered step. Both
// go through the same Sequencer, which is what keeps reply and push from racing.
export interface SeqUnit {
  seq: Seq;
  // Must be synchronous and total (apply everything or throw) — a half-applied unit
  // can reference-before-create.
  run: () => void;
}

// Applies units strictly in seq order, regardless of arrival order or which path
// (reply or push) delivered them. Out-of-order units are buffered until the gap fills;
// duplicates/replays (seq already applied) are dropped. A gap that never fills is a
// desync — `waiting` stays true and the caller should resync from `lastApplied`.
export class Sequencer {
  #last: Seq = 0;
  #pending = new Map<Seq, SeqUnit>();

  ingest(unit: SeqUnit): void {
    if (unit.seq <= this.#last) return; // already applied — dup or replay
    this.#pending.set(unit.seq, unit);
    while (this.#pending.has(this.#last + 1)) {
      const next = this.#pending.get(this.#last + 1)!;
      this.#pending.delete(next.seq);
      next.run();
      this.#last = next.seq;
    }
  }

  // seq of the last unit actually applied. Resync requests start from here.
  get lastApplied(): Seq {
    return this.#last;
  }

  // true when units are buffered ahead of a gap (i.e. we're waiting on a missing seq).
  // Transient during normal delivery; persistent means desync.
  get waiting(): boolean {
    return this.#pending.size > 0;
  }
}
