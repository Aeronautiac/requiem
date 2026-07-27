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
  OutputData,
  ProsecutionPhaseView,
  ServerInput,
  ServerOutput,
} from "./bindings";
import { slotKeyToString } from "./bindings";
import {
  actorLabel,
  GameState,
  phaseAnnouncement,
  phaseViewEqual,
  playerLabel,
} from "./game_state.svelte";
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

// Something this client sent and is still owed an answer for.
type Waiter = {
  // What we sent, kept only to check the reply against — see #settle.
  kind: "Action" | "Control";
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
    connection.onBatch((batch: ServerOutput) => this.#ingest(batch));
  }

  // A batch that throws mid-apply is unrecoverable and must say so.
  //
  // There is no continuing past it. Client state is cumulative — every command assumes its
  // predecessors landed — so a half-applied batch has already corrupted the state, and running
  // the next one on top would trade a visible failure for a silent wrong one. The Sequencer
  // agrees by construction: it does not advance its counter past a unit that threw, so the gap
  // never fills and nothing is ever applied again.
  //
  // What was missing is only that any of this was VISIBLE. The handler is invoked from a
  // promise continuation, so the throw became an unhandled rejection: the pipe stopped, the UI
  // quietly stopped updating, and anything awaiting a reply waited forever. Catching it here
  // settles those waiters and puts the client into an explicit desynced state. Recovery is a
  // reconnect, which replays the log from the start into fresh state.
  #ingest(output: ServerOutput) {
    try {
      this.#seq.ingest({
        seq: output.seq_num,
        run: () => this.#apply_output(output.data),
      });
    } catch (error) {
      console.error("failed to apply output", output.seq_num, error);
      this.abandon(
        "The client lost track of the game state and cannot continue. Please reconnect.",
      );
    }
  }

  // Two channels, one order. Commands carry the game; profiles carry what the SERVER knows about
  // who is playing each slot. They share a sequence counter precisely so a profile can never be
  // applied before the MapPlayer that introduced its slot.
  #apply_output(data: OutputData) {
    if ("Profiles" in data) {
      this.game.apply_profiles(data.Profiles);
      return;
    }
    this.#apply_batch(data.Batch);
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

    // Nothing is read off a successful response. Every fact it could carry arrives as a command,
    // which is what makes state reconstructible by any client and after any reconnect — a
    // response reaches only the connection that asked.
    waiter.settle();
  }

  // Fire this client's own action. Resolves to an error string on failure (for UX) or void on
  // success, once the reply has been applied in seq order — never before, so a caller can
  // trust that state reflects its action by the time it hears back.
  dispatch(request: ActionRequest): Promise<string | void> {
    return this.#submit({ Action: request });
  }

  // Game administration: teardown, key management, naming. Same reply path as an action.
  control(control: GameControl): Promise<string | void> {
    return this.#submit({ Control: control });
  }

  #submit(input: ServerInput): Promise<string | void> {
    return new Promise<string | void>((resolve) => {
      // Queued BEFORE the send so a reply that arrives immediately still finds its waiter.
      this.#waiting.push({
        kind: "Action" in input ? "Action" : "Control",
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
    if (!this.game.view_receives(recipient, this.#selected_view())) return;
    void this.host.notify({ title, body });
  }

  // The view the user is currently looking at. "Admin" selects the System view.
  #selected_view(): string {
    return this.ui.viewer === "Admin" ? "System" : this.ui.viewer;
  }

  quit(): void {
    this.host.quit();
  }

  #name(key: string): string {
    return playerLabel(key, this.game.players);
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
      // The command names only the kidnapping, so the victim is a lookup. Order-independent: a
      // reveal marks the tracked kidnapping rather than deleting it.
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
      // Diffed against the SELECTED view, since that is the only one that can toast anyway;
      // notify() then re-checks that the view actually receives this command.
      const view = this.game.views.get(this.#selected_view());
      const prev = view?.prosecutions.get(slotKeyToString(up.prosecution_id));
      if (view && (!prev || !phaseViewEqual(prev.phase, up.phase))) {
        this.notify(recipient, "Prosecution", this.#prosecution_text(up.prosecutor_display, up.defendant_display, up.phase, false));
      }
    } else if ("CloseProsecution" in cmd) {
      // Only toast if this view knew the prosecution; use its last-held displays/phase.
      const prev = this.game.views
        .get(this.#selected_view())
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
    return phaseAnnouncement(
      phase,
      actorLabel(prosecutor_display, players),
      actorLabel(defendant_display, players),
      ended,
    );
  }
}
