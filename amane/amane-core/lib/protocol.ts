// The contract between amane and whatever is feeding it. Defined in terms of yagami's wire
// shapes (see bindings.ts) because that is the real protocol; a host with no server behind it
// spoofs those shapes rather than amane growing a second contract.
//
// ONE stream, not two. A reply to this client's own input and the commands that input caused
// arrive TOGETHER in a single Batch and are applied as one step, commands FIRST and the response
// after — the response routinely names things its own commands create. Splitting them across a
// promise and a push stream would let them race, so the transport surfaces batches only, and a
// caller waiting on its own input is resolved from inside the ordered apply, never ahead of it.

import type { ServerInput, ServerOutput } from "../bindings";

// Why a submission produced no value. A VALUE, not a sentence: the render site turns it into copy
// through the strings config, so nothing in the pipeline holds a word of English.
export type ExecError =
  | { kind: "denied" }
  | { kind: "crashed" }
  | { kind: "desync" }
  // Refused on its own terms. `code` is the enum variant it refused with, which a render site may
  // recognise or fall back to showing raw.
  | { kind: "refused"; code: string };

export type Reply<T> = { ok: true; value: T } | { ok: false; error: ExecError };

// Server-stamped, per-connection, strictly +1 from 1. 0 means "nothing applied yet".
export type Seq = number;

// Deliberately NOT named `Notification`: that shadows the DOM global a web host uses to raise
// one, and this is the desktop popup where the in-app "Notifications" channel is the persistent
// log. The mechanism lives behind the router; deciding WHAT is toast-worthy stays in the session.
export interface Toast {
  title: string;
  body: string;
}

// One live connection to ONE game. A client displays the platform, not a single game, so this is
// created and destroyed as the player joins and leaves games — it is not the app.
export interface GameConnection {
  // Fire-and-forget: the reply comes back on the batch stream, not from here. Callers that need
  // to know how their input went register a waiter with the client, resolved when the batch
  // carrying the reply is applied in seq order.
  send(input: ServerInput): void;

  // This client's own replies AND everything caused by other clients or the server's own ticks.
  // Returns an unsubscribe function.
  onBatch(handler: (batch: ServerOutput) => void): () => void;

  // Idempotent — closing an already-closed connection is not an error.
  close(): void;
}

// The key is durable and per-game, handed out of band.
export interface GameTarget {
  gameId: number;
  key: string;
}

// Cross-game operations. Optional on the host context because a host may have no platform at all;
// absent means the UI simply doesn't offer it, the same context-injection rule as `canQuit`.
export interface PlatformApi {
  createGame(platformKey: string): Promise<{ game_id: number; admin_key: string }>;
  endGame(gameId: number, platformKey: string): Promise<void>;
}

// What a host must provide. Everything platform-shaped lives behind this, and amane never reaches
// past it — no platform checks, no feature sniffing.
export interface HostContext {
  // Rejects if the credential is refused; see the retry rule in the websocket transport (a 4xx is
  // final, a dead socket is retryable).
  connect(target: GameTarget): Promise<GameConnection>;

  // Fire-and-forget: implementations own their permission handshake and swallow failure, since a
  // denied prompt is UX rather than an error that should reach the state pipe. Host-level rather
  // than per-connection, because it outlives any game.
  notify(toast: Toast): Promise<void>;

  // Present only where there is a server to talk to.
  readonly platform?: PlatformApi;

  // Set when this host has exactly ONE game and no credential to choose it with: the client joins
  // at startup and never shows the join screen. A host talking to a real server leaves this
  // undefined, because there choosing a game IS the platform screen's job.
  readonly implicitGame?: GameTarget;

  // A browser tab cannot close itself the way a desktop window can. The UI gates on this rather
  // than guessing at the environment.
  readonly canQuit: boolean;
  quit(): void;
}

// One seq-ordered unit of client state change.
export interface SeqUnit {
  seq: Seq;
  // Must be synchronous and total (apply everything or throw) — a half-applied unit can
  // reference-before-create.
  run: () => void;
}

// Out-of-order units are buffered until the gap fills; already-applied seqs are dropped. A gap
// that never fills is a desync: `waiting` stays true and the caller should resync from
// `lastApplied`.
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

  // Resync requests start from here.
  get lastApplied(): Seq {
    return this.#last;
  }

  // Transient during normal delivery; persistent means desync.
  get waiting(): boolean {
    return this.#pending.size > 0;
  }
}
