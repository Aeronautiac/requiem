// The client for ONE game: it owns a connection and the two state layers underneath it —
// `game` (applied world state) and `ui` (view/selection state). It's the one place that can
// see the connection, the applied state, AND the currently-selected view at once, so the
// things that need all three live here and GameState stays pure state-apply:
//   - "talk to server + route replies": dispatch + the seq-ordered Sequencer.
//   - view-scoped native notifications: the client scans each batch it applies and raises a
//     toast for the view the user is actually looking at.
//
// One of these exists per joined game and is discarded on leaving. PlatformState owns the
// lifecycle; this type knows nothing about joining, leaving, or other games.
import { Sequencer } from "./lib/protocol";
import type { GameConnection, HostContext } from "./lib/protocol";
import type {
  ActionRequest,
  ActorDisplay,
  Batch,
  CommandPayload,
  CommandRecipient,
  ControlOutcome,
  ExecOutcome,
  GameControl,
  ProsecutionPhaseView,
  ServerInput,
  ServerOutput,
} from "./bindings";
import { slotKeyToString } from "./bindings";
import { actorLabel, GameState, phaseViewEqual, recipientToView } from "./game_state.svelte";
import { UiState } from "./ui_state.svelte";

export const CLIENT_KEY = Symbol("client");

// A control's outcome as a UX string, or nothing when it succeeded. The refusals are all
// authority failures, so they get plain readings rather than the enum name.
function controlError(outcome: ControlOutcome): string | void {
  if (outcome === "Denied") return "You are not an administrator of this game.";
  if ("Err" in outcome) {
    switch (outcome.Err) {
      case "KeyNotFound":
        return "That key does not exist.";
      case "CannotActOnSelf":
        return "You cannot change your own key.";
      case "RequiresSupervise":
        return "Only a supervisor can change another administrator's key.";
      case "CannotGrantSupervise":
        return "Only a supervisor can grant the supervisor capability.";
    }
  }
}

// Something this client sent and is still owed an answer for. `args` is UI-side context the
// response handler needs (it isn't on the wire), carried here until the reply lands.
type Waiter = {
  // What we sent, kept only to check the reply against — see #settle.
  kind: "Action" | "Control";
  args?: Record<string, unknown>;
  settle: (error: string | void) => void;
};

export class ClientState {
  readonly connection: GameConnection;
  readonly host: HostContext;
  readonly ui = new UiState();
  readonly game = new GameState();
  // Our own replies AND everything caused by other clients funnel through one ordered pipe,
  // so they can never race into a desync.
  #seq = new Sequencer();
  // Inputs awaiting a reply, oldest first. Correlation is positional rather than by id: the
  // server replies to a connection strictly in the order that connection submitted, so the
  // n-th reply belongs to the n-th thing we sent. That is the only reason no correlation id
  // is needed, so it is load-bearing — see the echo check in #settle.
  #waiting: Waiter[] = [];

  constructor(connection: GameConnection, host: HostContext) {
    this.connection = connection;
    this.host = host;
    connection.onBatch((batch: ServerOutput) =>
      this.#seq.ingest({
        seq: batch.seq_num,
        run: () => this.#apply_batch(batch.data.Batch),
      }),
    );
  }

  // Apply one batch as a single ordered step: commands first, then the reply if this batch
  // carried one. That order is required, not incidental — a response routinely names things
  // its own commands create, so it can only be resolved once they are in place.
  #apply_batch({ commands, response }: Batch) {
    this.#apply(commands);
    if (response) this.#settle(response.input, response.output);
  }

  // Apply commands to game state and raise any view-scoped toasts for them. Notifications are
  // derived here (not in GameState) so GameState never depends on the router or the UI; they
  // fire BEFORE the apply so a command whose apply consumes state (e.g. KidnapReveal clearing
  // the kidnapping) can still be resolved for the toast text.
  #apply(commands: CommandPayload[]) {
    for (const c of commands) this.#maybe_notify(c);
    this.game.apply_batch(commands);
  }

  // Hand one reply back to whoever sent it, turning the outcome into the error string the UI
  // expects (or nothing, on success).
  #settle(input: ServerInput, output: ExecOutcome) {
    const waiter = this.#waiting.shift();
    if (!waiter) return; // a reply nobody is waiting on — not ours to route

    // Cheap guard on the assumption the whole positional scheme rests on. The server echoes
    // the input it answered, so if its KIND disagrees with what we queued, our replies and
    // our waiters have drifted out of step and every subsequent one would be misrouted —
    // resolved against the wrong caller, with the wrong `args`. Fail loudly instead.
    const kind = "Action" in input ? "Action" : "Control";
    if (kind !== waiter.kind) {
      this.abandon("Lost track of which reply belongs to which request. Please reconnect.");
      waiter.settle("Reply mismatch — the client and server are out of step.");
      return;
    }

    if ("Control" in output) {
      waiter.settle(controlError(output.Control));
      return;
    }

    const outcome = output.Action;
    if (outcome === "Crashed") return waiter.settle("The engine has crashed.");
    if (outcome === "Denied") return waiter.settle("You are not permitted to do that.");
    if ("Err" in outcome) return waiter.settle(String(outcome.Err));

    // Commands for this batch are already applied, so the response resolves against them.
    if ("Action" in input) this.game.handle_response(outcome.Ok, waiter.args);
    waiter.settle();
  }

  // Fire this client's own action. Resolves to an error string on failure (for UX) or void on
  // success, once the reply has been applied in seq order — never before, so a caller can
  // trust that state reflects its action by the time it hears back.
  dispatch(
    request: ActionRequest,
    args?: Record<string, unknown>,
  ): Promise<string | void> {
    return this.#submit({ Action: request }, args);
  }

  // Game administration: teardown and key management. Same reply path as an action.
  control(control: GameControl): Promise<string | void> {
    return this.#submit({ Control: control });
  }

  #submit(input: ServerInput, args?: Record<string, unknown>): Promise<string | void> {
    return new Promise<string | void>((resolve) => {
      // Queued BEFORE the send so a reply that arrives immediately still finds its waiter.
      this.#waiting.push({
        kind: "Action" in input ? "Action" : "Control",
        args,
        settle: resolve,
      });
      this.connection.send(input);
    });
  }

  // Fail every outstanding waiter. A dropped connection means their replies are never coming,
  // and a promise that never settles is a UI stuck on "in progress" forever.
  abandon(reason: string) {
    const waiting = this.#waiting;
    this.#waiting = [];
    for (const waiter of waiting) waiter.settle(reason);
  }

  // Raise a native OS/browser toast, but only for the view the user is currently looking at —
  // this client holds every view, so without this gate it would toast for all of them. The
  // selected viewer is `ui.viewer` ("Admin" selects the System view). Best-effort UX: failure
  // is swallowed inside the router.
  notify(recipient: CommandRecipient, title: string, body: string): void {
    const selected = this.ui.viewer === "Admin" ? "System" : this.ui.viewer;
    if (recipientToView(recipient) !== selected) return;
    void this.host.notify({ title, body });
  }

  quit(): void {
    this.host.quit();
  }

  #name(key: string): string {
    return this.game.players.get(key)?.display_name ?? "Unknown";
  }

  // Which world events warrant a toast, and their text. Mirrors the in-app Announcement copy
  // in ChannelView; reveals are deliberately excluded (you triggered those yourself).
  #maybe_notify({ recipient, cmd }: CommandPayload): void {
    if ("Death" in cmd) {
      this.notify(recipient, "Death", `${this.#name(slotKeyToString(cmd.Death.target_id))} has died.`);
    } else if ("AnonymousAnnouncement" in cmd) {
      this.notify(recipient, "Anonymous Announcement", cmd.AnonymousAnnouncement.content);
    } else if ("Kidnapping" in cmd) {
      this.notify(recipient, "Kidnapping", `${this.#name(slotKeyToString(cmd.Kidnapping.target_id))} has been kidnapped.`);
    } else if ("KidnapReveal" in cmd) {
      // Resolve the victim from the still-present kidnapping (apply hasn't cleared it yet).
      const victim = this.game.kidnappings.get(slotKeyToString(cmd.KidnapReveal.kidnapping_id))?.victim;
      const victimName = victim ? this.#name(victim) : "the victim";
      const kidnapper = cmd.KidnapReveal.kidnapper;
      this.notify(
        recipient,
        "Kidnap Reveal",
        kidnapper
          ? `Authorities have recovered ${victimName}, and ${this.#name(slotKeyToString(kidnapper))} was revealed as the kidnapper.`
          : `Authorities have recovered ${victimName}, but the kidnapper stayed anonymous.`,
      );
    } else if ("PseudocideRevival" in cmd) {
      this.notify(recipient, "Revival", `${this.#name(slotKeyToString(cmd.PseudocideRevival.target_id))} is alive.`);
    } else if ("RoleUpdate" in cmd && typeof recipient !== "string") {
      // Personal info: only the player's own copy (an Actor recipient) toasts, never the System mirror.
      this.notify(recipient, "Role", `Your role is now ${cmd.RoleUpdate.role}.`);
    } else if ("TrueNameUpdate" in cmd && typeof recipient !== "string") {
      this.notify(recipient, "True Name", `Your true name is now ${cmd.TrueNameUpdate.true_name}.`);
    } else if ("UpdateProsecution" in cmd) {
      // Prosecution is frontend-derived: toast on the same condition game_state emits a news event
      // — a new prosecution or a phase change. Runs before apply, so `prev` is the old snapshot.
      const up = cmd.UpdateProsecution;
      const view = this.game.views.get(recipientToView(recipient) ?? "");
      const prev = view?.prosecutions.get(slotKeyToString(up.prosecution_id));
      if (view && (!prev || !phaseViewEqual(prev.phase, up.phase))) {
        this.notify(recipient, "Prosecution", this.#prosecution_text(up.prosecutor_display, up.defendant_display, up.phase, false));
      }
    } else if ("CloseProsecution" in cmd) {
      // Only toast if this view knew the prosecution; use its last-held displays/phase.
      const prev = this.game.views
        .get(recipientToView(recipient) ?? "")
        ?.prosecutions.get(slotKeyToString(cmd.CloseProsecution.prosecution_id));
      if (prev) {
        this.notify(recipient, "Prosecution Ended", this.#prosecution_text(prev.prosecutor_display, prev.defendant_display, prev.phase, true));
      }
    }
  }

  // Mirrors prosecution_event_text in ChannelView: the phase-appropriate line for a prosecution.
  #prosecution_text(
    prosecutor_display: ActorDisplay,
    defendant_display: ActorDisplay,
    phase: ProsecutionPhaseView,
    ended: boolean,
  ): string {
    const players = this.game.players;
    const defendant = actorLabel(defendant_display, players);
    if (ended) return `The prosecution of ${defendant} has ended.`;
    const prosecutor = actorLabel(prosecutor_display, players);
    if (phase === "Custody") return `${prosecutor} is prosecuting ${defendant}.`;
    if (phase === "Voting") return `The verdict vote for ${defendant} has begun.`;
    if (phase.Trial === "Prosecutor") return `The trial of ${defendant} has begun — the prosecution presents.`;
    if (phase.Trial === "Defense") return `In the trial of ${defendant}, the defense presents.`;
    return `The trial of ${defendant} has entered debate.`;
  }
}
