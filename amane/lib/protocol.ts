// ////////////////////////////////////////////////////////////
// STREAMING PROTOCOL SEAM
// ////////////////////////////////////////////////////////////
//
// The contract between amane and whatever is feeding it. Defined in terms of yagami's
// wire shapes (see bindings.ts) because that is the real protocol; a host with no server
// behind it spoofs those shapes rather than amane growing a second contract.
//
// ONE stream, not two. A reply to this client's own input and the commands that input
// caused arrive TOGETHER in a single Batch and are applied as one step, commands FIRST and
// the response after — the response routinely names things its own commands create, so it
// can only be resolved against state those commands have already established. Splitting
// them across a promise and a push stream would let them race, so the transport surfaces
// batches only, and a caller waiting on its own input is resolved from inside the ordered
// apply, never ahead of it.
//
// Ordering is carried by `seq_num`: per-connection, strictly +1, starting at 1. Batches are
// applied in that order via the Sequencer regardless of what order they physically arrive.

import type { ServerInput, ServerOutput } from "../bindings";

// Server-stamped, per-client, strictly increasing (by 1). seq 0 means "nothing applied
// yet". The single source of truth for the order batches must be applied in.
export type Seq = number;

// A native OS/browser toast request. Deliberately NOT named `Notification` to avoid
// shadowing the DOM global (which a web host uses to actually raise one) and to keep it
// distinct from the in-app "Notifications" info channel — this is the desktop popup, that
// is the persistent log. The platform mechanism lives behind the router; deciding WHAT is
// toast-worthy stays in the client model.
export interface Toast {
  title: string;
  body: string;
}

// One live connection to ONE game. A client displays the platform, not a single game, so
// this is created and destroyed as the player joins and leaves games — it is not the app.
export interface GameConnection {
  // Fire-and-forget: the reply comes back on the batch stream, not from here. Callers that
  // need to know how their input went register a waiter with the client, which is resolved
  // when the batch carrying the reply is applied in seq order.
  send(input: ServerInput): void;

  // Subscribe to the batch stream: this client's own replies AND everything caused by other
  // clients or by the server's own ticks. Returns an unsubscribe function.
  onBatch(handler: (batch: ServerOutput) => void): () => void;

  // Leave the game. Idempotent — closing an already-closed connection is not an error.
  close(): void;
}

// Which game to join, and with what. The key is durable and per-game, handed out of band.
export interface GameTarget {
  gameId: number;
  key: string;
}

// Cross-game operations, which are the PLATFORM's rather than any game's. Optional on the
// host context because a host may not have a platform at all: armonia runs one engine
// in-process with nothing to create or destroy. Absent means the UI simply doesn't offer it
// — the same context-injection rule as `canQuit`, not a special case.
export interface PlatformApi {
  createGame(platformKey: string): Promise<{ game_id: number; admin_key: string }>;
  endGame(gameId: number, platformKey: string): Promise<void>;
}

// What a host must provide. Everything platform-shaped lives behind this, and amane never
// reaches past it — no platform checks, no feature sniffing.
export interface HostContext {
  // Open a connection to one game. Rejects if the credential is refused; see the retry rule
  // in the websocket transport (a 4xx is final, a dead socket is retryable).
  connect(target: GameTarget): Promise<GameConnection>;

  // Raise a native OS/browser notification. Fire-and-forget: implementations own their
  // permission handshake and swallow failure (a denied prompt is UX, never an error that
  // reaches the state pipe). Host-level rather than per-connection: it outlives any game.
  notify(toast: Toast): Promise<void>;

  // Present only where there is a server to talk to.
  readonly platform?: PlatformApi;

  // Set when this host has exactly ONE game and no credential to choose it with, so there is
  // nothing for a player to fill in: the client joins it at startup and never shows the join
  // screen. armonia is the case — it hosts a single engine in-process. A host talking to a
  // real server leaves this undefined, because there choosing a game IS the platform screen's
  // job. Injected rather than sniffed, same as `platform` and `canQuit`.
  readonly implicitGame?: GameTarget;

  // Whether quitting is even a thing here — a browser tab cannot close itself the way a
  // desktop window can. The UI gates on this rather than guessing at the environment.
  readonly canQuit: boolean;
  quit(): void;
}

// One seq-ordered unit of client state change. `run` applies its whole effect: the batch's
// commands, then its response if it carried one.
export interface SeqUnit {
  seq: Seq;
  // Must be synchronous and total (apply everything or throw) — a half-applied unit can
  // reference-before-create.
  run: () => void;
}

// Applies units strictly in seq order, regardless of arrival order. Out-of-order units are
// buffered until the gap fills; duplicates/replays (seq already applied) are dropped. A gap
// that never fills is a desync — `waiting` stays true and the caller should resync from
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
