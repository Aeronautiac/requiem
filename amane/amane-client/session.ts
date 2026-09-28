// One joined game: the connection, the fold into a Game, and correlation of replies to what this
// session sent. One exists per join and is dropped on leaving; the Client owns that lifecycle.
import type {
  ActionRequest,
  ActionResponse,
  AdminControl,
  Batch,
  ControlResponse,
  ExecOutcome,
  PrivilegeSet,
  ServerInput,
} from "./bindings.ts";
import { Game } from "./game/game.ts";
import type { Observer } from "./game/game.ts";
import type { GameConnection, Reply } from "./host.ts";

// For a surface to key per-session state on, so a rejoin of the same game still starts fresh.
let next_id = 1;

type Waiter = (outcome: ExecOutcome | null) => void;

export class Session {
  readonly id = next_id++;
  readonly game_id: number;
  readonly key: string;
  // Replaced by every Initialize batch, never cleared in place.
  game: Game;
  // What our own key permits, as the server states it. Null until stated, and null reads as
  // "nothing", which is the safe way round. A fact about this connection, not the game.
  privileges: PrivilegeSet | null = null;
  // Bumps the client's version. Called after every batch; a surface that changes game state itself
  // (a view's outbox) calls it too, so what it changed is drawn.
  readonly changed: () => void;

  #connection: GameConnection;
  #observers: Observer[];
  // Oldest first. Correlation is positional: the server replies to a connection strictly in the
  // order it submitted, so the n-th reply belongs to the n-th thing sent.
  #waiting: Waiter[] = [];
  // Between an Initialize terminal and the next Live terminal: everything applied is history.
  #catching_up = false;

  constructor(
    game_id: number,
    key: string,
    connection: GameConnection,
    observers: Observer[],
    changed: () => void,
  ) {
    this.game_id = game_id;
    this.key = key;
    this.#connection = connection;
    this.#observers = observers;
    this.changed = changed;
    this.game = new Game(observers);
    connection.onBatch((batch) => this.#ingest(batch));
  }

  // A large batch may arrive split: a terminal chunk ("Initialize" or Live) followed by
  // "Continuation" chunks that extend it. Only a terminal changes the mode.
  //
  // A command that fails to apply is caught inside the Game, one delivery at a time, so a bad
  // command never stops the batch and the reply below still settles.
  #ingest(batch: Batch) {
    const kind = batch.kind;
    const live = kind !== "Initialize" && kind !== "Continuation";
    if (kind === "Initialize") {
      this.game = new Game(this.#observers);
      this.privileges = null;
      this.#catching_up = true;
    }
    // A Live terminal ends catch-up before its own outputs: they are news.
    if (live) this.#catching_up = false;

    for (const out of batch.outputs) {
      // This connection's own privileges: a connection-wide fact, read here. It has no recipients,
      // so the Game routes it nowhere.
      if ("Server" in out.data && "Privileges" in out.data.Server) {
        this.privileges = out.data.Server.Privileges;
      }
      this.game.apply_output(out, this.#catching_up);
    }

    // A Live terminal may carry the reply to one of this connection's own inputs, settled only
    // after its commands are applied.
    if (live && kind.Live) this.#waiting.shift()?.(kind.Live.response);
    this.changed();
  }

  // Everything this session sends goes through here. Resolves once the reply has been applied,
  // never before, so state already reflects the input when the caller hears back. Null means the
  // reply is never coming.
  #submit(input: ServerInput): Promise<ExecOutcome | null> {
    return new Promise((resolve) => {
      // Queued before the send, so a reply that arrives at once still finds its waiter.
      this.#waiting.push(resolve);
      this.#connection.send(input);
    });
  }

  async submit_action(request: ActionRequest): Promise<Reply<ActionResponse>> {
    const outcome = await this.#submit({ Action: request });
    // The wrong SHAPE back means replies and waiters have drifted, and every later reply would
    // land on the wrong caller.
    if (outcome === null || !("Action" in outcome)) return this.#drift();

    const action = outcome.Action;
    if (action === "EnginePanic") return { ok: false, error: { kind: "crashed" } };
    if (action === "Denied") return { ok: false, error: { kind: "denied" } };
    if ("Err" in action) return { ok: false, error: { kind: "refused", code: action.Err } };
    return { ok: true, value: action.Ok };
  }

  async submit_control(control: AdminControl): Promise<Reply<ControlResponse>> {
    const outcome = await this.#submit({ Control: control });
    if (outcome === null || !("Control" in outcome)) return this.#drift();

    const result = outcome.Control;
    if (result === "Denied") return { ok: false, error: { kind: "denied" } };
    if ("Err" in result) return { ok: false, error: { kind: "refused", code: result.Err } };
    return { ok: true, value: result.Ok };
  }

  #drift<T>(): Reply<T> {
    this.abandon();
    return { ok: false, error: { kind: "desync" } };
  }

  // Fail every outstanding waiter: their replies are never coming, and a promise that never
  // settles is a UI stuck on "in progress" forever.
  abandon() {
    const waiting = this.#waiting;
    this.#waiting = [];
    for (const settle of waiting) settle(null);
  }

  close() {
    this.#connection.close();
    this.abandon();
  }
}
