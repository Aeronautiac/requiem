// What a host gives the core: transport and the environment's few abilities, nothing more. The
// protocol on top (every REST endpoint, tickets, reply correlation) lives in the core, once, so no
// host re-implements it.
//
// A host is the app's entry: it builds one of these, builds the Client with it, and mounts a
// surface. Differences between hosts are capabilities here, never branches in a surface.
import type { Batch, ServerInput } from "./bindings.ts";

// Why a call produced no value. A VALUE, not a sentence: the render site turns it into words
// through `execErrorText`, so nothing in the pipeline holds English.
export type ExecError =
  | { kind: "denied" }
  | { kind: "crashed" }
  | { kind: "desync" }
  // Refused on its own terms. `code` is the enum variant it refused with.
  | { kind: "refused"; code: string }
  // A REST endpoint refused, with yagami's ServerError variant.
  | { kind: "server"; code: string }
  // The request never got an answer.
  | { kind: "network" };

export type Reply<T> = { ok: true; value: T } | { ok: false; error: ExecError };

// A desktop/OS notification. Not named `Notification`, which is the DOM global a web host raises
// one with.
export type Toast = { title: string; body: string };

// One live connection to ONE game.
//
// ONE stream, not two. A reply to this client's own input and the commands that input caused
// arrive together in a single Batch, commands first. Splitting them across a promise and a push
// stream would let them race, so the transport surfaces batches only.
export interface GameConnection {
  // Fire-and-forget: the reply comes back on the batch stream.
  send(input: ServerInput): void;
  // Batches arrive in order, so they are applied as they arrive. One handler at a time.
  onBatch(handler: (batch: Batch) => void): void;
  // Called once if the connection dies on its own (server gone, key revoked, network), never for a
  // deliberate `close`. The reason is for display.
  onDropped(handler: (reason: string) => void): void;
  // Idempotent.
  close(): void;
}

export type HttpResponse = { status: number; body: string };

export interface Host {
  // One HTTP request against the server. `body` is JSON-encoded when present. Resolves with
  // whatever status came back; rejects only when there was no answer at all. Cookies are the
  // host's business: a browser sends its own, anything else keeps a jar.
  request(method: string, path: string, body?: unknown): Promise<HttpResponse>;

  // Open the game socket for a ticket the core already obtained. Rejects if the socket never
  // opens.
  open_socket(game_id: number, ticket: string): Promise<GameConnection>;

  // Best-effort: a denied permission or a missing notification system is swallowed here.
  notify(toast: Toast): Promise<void>;

  // Absent where the environment can't close itself (a browser tab).
  quit?: () => void;
}
