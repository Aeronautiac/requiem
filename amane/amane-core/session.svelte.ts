// One joined game: the connection, the apply pipe, and the two state layers underneath — `game`
// (what was delivered) and `ui` (what is selected).
//
// The one place that can see the connection, the applied state AND the view on screen at once, so
// whatever needs all three lives here and GameState stays a pure fold over commands.
//
// One of these exists per joined game and is discarded on leaving. ClientState owns the lifecycle;
// this type knows nothing about joining, leaving, or other games.
import type { GameConnection, HostContext, Reply } from "./lib/protocol";
import type {
  ActionRequest,
  ActionResponse,
  ActorKey,
  AdminControl,
  Batch,
  ControlResponse,
  ExecOutcome,
  PrivilegeSet,
  ServerInput,
  ServerOutput,
} from "./bindings";
import { GameState } from "./game/state.svelte";
import { UiState } from "./ui_state.svelte";

export const SESSION_KEY = Symbol("session");

// Something this session sent and is still owed an answer for.
type Waiter = (outcome: ExecOutcome | null) => void;

export class SessionState {
  readonly connection: GameConnection;
  readonly host: HostContext;
  readonly ui = new UiState();
  // One GameState for the life of the session, resynced IN PLACE rather than swapped whole. The UI
  // binds to this exact instance through context at mount, so replacing it would strand every
  // component on the stale pre-resync layers. An "Initialize" batch instead clears the same
  // instance's reactive internals and folds the catch-up into them.
  readonly game = new GameState();
  // What our own key permits, as the server states it. Null until it is stated — so "null" reads
  // as "nothing", which is the safe way round: the UI offers nothing until told otherwise rather
  // than offering everything until corrected.
  //
  // Session-level rather than per-view, and deliberately not in GameState: it is a fact about this
  // CONNECTION, not about the game, and no view may be built out of it.
  privileges = $state<PrivilegeSet | null>(null);
  // Oldest first. Correlation is positional rather than by id: the server replies to a connection
  // strictly in the order that connection submitted, so the n-th reply belongs to the n-th thing
  // we sent. Load-bearing — see the shape check in submit_action and submit_control.
  #waiting: Waiter[] = [];
  // True only while the outputs of a catch-up (Initialize) batch are being applied — a fresh
  // attach's replay, or the resync every connection gets after a time-travel rewind. Toasts are
  // suppressed for exactly that window: everything a connection is owed arrives at once during
  // catch-up, and nobody wants the whole history buzzing their lock screen. It is not a latch — a
  // resync can happen any time, so the flag lives and dies with each Initialize batch.
  #catching_up = false;

  constructor(connection: GameConnection, host: HostContext) {
    this.connection = connection;
    this.host = host;
    connection.onBatch((batch: Batch) => this.#ingest(batch));
    this.#wire();
  }

  // Route toasts from the current game state. Wired on construction and again after every swap,
  // because a new GameState has no notifier until told who the human on screen is — and which view
  // they are looking at. A handler composes a toast and hands it here; this decides whether the
  // human sees it: only for the view on screen and only outside a catch-up window — a toast for a
  // view the user is not looking at, or one replayed during a resync, is not something to interrupt
  // them with. Best-effort: a rejected or unsupported notification is swallowed inside the host.
  #wire() {
    this.game.set_notifier((view, toast) => {
      if (!this.#catching_up && view === this.game.views.get(this.#selected_key())) {
        void this.host.notify(toast);
      }
    });
  }

  // Batches arrive in order — the server drives every connection's outbox with one task and a
  // reliable, ordered transport carries them — so no sequencing is needed: a batch is applied when
  // it arrives, and an "Initialize" batch replaces the state and rebuilds it from its outputs.
  //
  // A batch that throws mid-apply is unrecoverable, and the catch is what makes that VISIBLE.
  // Client state is cumulative, so a half-applied batch has already corrupted it and running the
  // next on top would trade a visible failure for a silent wrong one. Recovery is a reconnect,
  // which replays the log from the start into fresh state.
  #ingest(batch: Batch) {
    console.log(batch);
    try {
      if (batch.kind === "Initialize") {
        // Reset is a session-level concern, not a game-state one: authority (privileges), the view
        // layers, and the catch-up window all belong to the connection and reset together. The same
        // GameState is cleared in place (the UI is bound to it) and rebuilds from the catch-up.
        this.privileges = null;
        this.game.reset();
        this.#catching_up = true;
      }
      for (const out of batch.outputs) this.#apply_output(out);
      if (batch.kind === "Initialize") {
        const sys = this.game.system_view();
        console.log(
          "[diag] initialize applied",
          batch.outputs.length,
          "views:",
          [...this.game.views.keys()],
          "players:",
          sys.players.size,
          "channels:",
          sys.channels.size,
          "abilities:",
          sys.abilities.size,
          "events:",
          sys.events.length,
          "viewports:",
          sys.viewports.size,
        );
        this.#catching_up = false;
      } else {
        const pair = batch.kind.Live;
        if (pair) this.#waiting.shift()?.(pair.response);
      }
    } catch (error) {
      console.error("failed to apply batch", error);
      this.abandon();
    }
  }

  #apply_output(out: ServerOutput) {
    // This connection's own privileges are a connection-wide fact, read here before the command
    // flows on through the single dispatch path (its Connection gate routes it to no view).
    const data = out.data;
    if ("Server" in data && "Privileges" in data.Server) {
      this.privileges = data.Server.Privileges;
    }
    let recipients = this.game.apply_output(out);
    if (this.#catching_up && recipients === 0 && "Engine" in data) {
      console.log(
        "[diag] engine cmd routed to 0 views:",
        Object.keys(data.Engine)[0],
        "gates:",
        JSON.stringify(out.view_gates),
      );
    }
  }

  // ---- what this key permits ----

  // Whether the admin surfaces are offered at all. Not a security boundary: the server denies a
  // control from a key without it whatever the client renders.
  get administers(): boolean {
    return this.privileges?.capabilities.includes("Administer") ?? false;
  }

  // Authority over OTHER administrators' keys, which is what decides whether a key form may offer
  // Supervise or aim at an admin's key at all.
  get supervises(): boolean {
    return this.privileges?.capabilities.includes("Supervise") ?? false;
  }

  // The actors this key may act as, or empty for a scope that names none individually. `All` is
  // deliberately not expanded into a list: it covers actors created later, which no list can.
  get actors(): ActorKey[] {
    const scope = this.privileges?.actors;
    return scope !== undefined && scope !== "All" ? scope.Only : [];
  }

  // Every view this connection may look through, admin's first. THE answer to "who can I be" — the
  // view picker offers exactly these, and the game screen renders nothing at all while it is empty,
  // because almost every component below reads a current view and there is no honest one to give
  // them.
  //
  // Read off the views that have actually received something rather than off `actors`: an actor is
  // only worth looking through once something has been delivered to it, and `All` names no actors
  // to enumerate. System is the admin view and appears under the name the UI uses for it, so it is
  // never offered twice.
  get viewers(): string[] {
    const actors = [...this.game.views.keys()]
      .filter((key) => key !== "System")
      .sort((a, b) => parseInt(a) - parseInt(b));
    return this.administers ? ["Admin", ...actors] : actors;
  }

  // ---- the one path in and out ----

  // Everything this session sends goes through here, and the reply comes back exactly as the
  // server gave it. Null means the reply is never coming.
  //
  // Nothing below this line knows what an Action is or what a Control is. That distinction belongs
  // to the two typed edges underneath, not to the correlation machinery.
  submit(input: ServerInput): Promise<ExecOutcome | null> {
    return new Promise<ExecOutcome | null>((resolve) => {
      // Queued BEFORE the send so a reply that arrives immediately still finds its waiter.
      this.#waiting.push(resolve);
      this.connection.send(input);
    });
  }

  // Resolves once the reply has been applied and never before, so a caller can trust that state
  // reflects its action by the time it hears back.
  async submit_action(request: ActionRequest): Promise<Reply<ActionResponse>> {
    const outcome = await this.submit({ Action: request });
    // Getting the wrong SHAPE back is the failure the positional scheme has to catch: replies and
    // waiters have drifted, so every subsequent one would resolve against the wrong caller.
    if (outcome === null || !("Action" in outcome)) return this.#drift();

    const action = outcome.Action;
    if (action === "EnginePanic") return { ok: false, error: { kind: "crashed" } };
    if (action === "Denied") return { ok: false, error: { kind: "denied" } };
    if ("Err" in action) return { ok: false, error: { kind: "refused", code: String(action.Err) } };
    return { ok: true, value: action.Ok };
  }

  // Teardown, key management, naming. Same path, other half of the union.
  async submit_control(control: AdminControl): Promise<Reply<ControlResponse>> {
    const outcome = await this.submit({ Control: control });
    if (outcome === null || !("Control" in outcome)) return this.#drift();

    const result = outcome.Control;
    if (result === "Denied") return { ok: false, error: { kind: "denied" } };
    if ("Err" in result) return { ok: false, error: { kind: "refused", code: String(result.Err) } };
    return { ok: true, value: result.Ok };
  }

  #drift<T>(): Reply<T> {
    this.abandon();
    return { ok: false, error: { kind: "desync" } };
  }

  // Fail every outstanding waiter. A dropped connection means their replies are never coming, and
  // a promise that never settles is a UI stuck on "in progress" forever.
  abandon() {
    const waiting = this.#waiting;
    this.#waiting = [];
    for (const settle of waiting) settle(null);
  }

  quit(): void {
    this.host.quit();
  }

  // ---- toasts ----

  // The view on screen. "Admin" is the UI's name for System.
  #selected_key(): string {
    return this.ui.viewer === "Admin" ? "System" : this.ui.viewer;
  }
}
