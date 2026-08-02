// One joined game: the connection, the ordered apply pipe, and the two state layers underneath —
// `game` (what was delivered) and `ui` (what is selected).
//
// The one place that can see the connection, the applied state AND the view on screen at once, so
// whatever needs all three lives here and GameState stays a pure fold over commands.
//
// One of these exists per joined game and is discarded on leaving. ClientState owns the lifecycle;
// this type knows nothing about joining, leaving, or other games.
import { Sequencer } from "./lib/protocol";
import type { GameConnection, HostContext, Reply } from "./lib/protocol";
import type {
  ActionRequest,
  ActionResponse,
  ActorKey,
  Batch,
  ControlResponse,
  ExecOutcome,
  GameControl,
  OutputData,
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
  readonly game = new GameState();
  // What our own key permits, as the server states it. Null until the first packet, which arrives
  // ahead of the catch-up — so the window is real but empty, and "null" reads as "nothing", which
  // is the safe way round: the UI offers nothing until told otherwise rather than offering
  // everything until corrected.
  //
  // Session-level rather than per-view, and deliberately not in GameState: it is a fact about this
  // CONNECTION, not about the game, and no view may be built out of it.
  privileges = $state<PrivilegeSet | null>(null);
  // Our own replies AND everything caused by other clients funnel through one ordered pipe, so
  // they can never race into a desync.
  #seq = new Sequencer();
  // Oldest first. Correlation is positional rather than by id: the server replies to a connection
  // strictly in the order that connection submitted, so the n-th reply belongs to the n-th thing
  // we sent. Load-bearing — see the shape check in submit_action and submit_control.
  #waiting: Waiter[] = [];
  // False until the first batch — the full catch-up replay — has been applied. Toasts are
  // suppressed for it: everything a connection is owed arrives at once on attach, and a player who
  // just joined does not want the whole history buzzing their lock screen. Every batch after is a
  // live push, which is exactly what a toast is for.
  #live = false;

  constructor(connection: GameConnection, host: HostContext) {
    this.connection = connection;
    this.host = host;
    connection.onBatch((batch: ServerOutput) => this.#ingest(batch));

    // A handler composes a toast and hands it here; this decides whether the human sees it. It is
    // raised only for the view on screen and only once we are live — a toast for a view the user
    // is not looking at, or one replayed during catch-up, is not something to interrupt them with.
    // Best-effort: a rejected or unsupported notification is swallowed inside the host.
    this.game.set_notifier((view, toast) => {
      if (this.#live && view === this.game.views.get(this.#selected_key())) {
        void this.host.notify(toast);
      }
    });
  }

  // A batch that throws mid-apply is unrecoverable, and the catch is what makes that VISIBLE.
  //
  // Client state is cumulative, so a half-applied batch has already corrupted it and running the
  // next on top would trade a visible failure for a silent wrong one. The Sequencer agrees by
  // construction: it does not advance past a unit that threw, so the gap never fills. But this
  // handler runs from a promise continuation, so without the catch the throw is an unhandled
  // rejection — the UI quietly stops updating and every waiter hangs forever. Recovery is a
  // reconnect, which replays the log from the start into fresh state.
  #ingest(output: ServerOutput) {
    try {
      this.#seq.ingest({
        seq: output.seq_num,
        run: () => this.#apply_output(output.data),
      });
    } catch (error) {
      console.error("failed to apply output", output.seq_num, error);
      this.abandon();
    }
  }

  // Two channels, one order. Commands carry the game; profiles carry what the SERVER knows about
  // who is playing each slot. They share a sequence counter precisely so a profile can never be
  // applied before the MapActor that introduced its slot.
  #apply_output(data: OutputData) {
    if ("Profiles" in data) {
      this.game.apply_profiles(data.Profiles);
      return;
    }
    if ("Privileges" in data) {
      this.#apply_privileges(data.Privileges);
      return;
    }
    this.#apply_batch(data.Batch);
  }

  // Replaced wholesale — the packet states the complete set, exactly as the controls that write it
  // do. Applied in seq order like everything else, so it can never be read against state from
  // before the change it describes.
  #apply_privileges(privileges: PrivilegeSet) {
    this.privileges = privileges;
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

  // Commands first, then the reply. That order is required, not incidental — a response routinely
  // names things its own commands create, so it can only be resolved once they are in place.
  //
  // Toasts raise themselves from inside the apply, wherever a handler calls `ctx.notify`. All this
  // has to do is mark us live once the catch-up batch is behind us — see #live and the notifier.
  #apply_batch({ commands, response }: Batch) {
    this.game.apply_batch(commands);
    if (response) this.#waiting.shift()?.(response.output);
    this.#live = true;
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

  // Resolves once the reply has been applied in seq order and never before, so a caller can trust
  // that state reflects its action by the time it hears back.
  async submit_action(request: ActionRequest): Promise<Reply<ActionResponse>> {
    const outcome = await this.submit({ Action: request });
    // Getting the wrong SHAPE back is the failure the positional scheme has to catch: replies and
    // waiters have drifted, so every subsequent one would resolve against the wrong caller.
    if (outcome === null || !("Action" in outcome)) return this.#drift();

    const action = outcome.Action;
    if (action === "Crashed") return { ok: false, error: { kind: "crashed" } };
    if (action === "Denied") return { ok: false, error: { kind: "denied" } };
    if ("Err" in action) return { ok: false, error: { kind: "refused", code: String(action.Err) } };
    return { ok: true, value: action.Ok };
  }

  // Teardown, key management, naming. Same path, other half of the union.
  async submit_control(control: GameControl): Promise<Reply<ControlResponse>> {
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
