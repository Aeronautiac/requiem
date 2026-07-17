// The top-level client: it owns the transport and the two state layers underneath it —
// `game` (applied world state) and `ui` (view/selection state). It's the one place that can
// see the router, the applied state, AND the currently-selected view at once, so the things
// that need all three live here and GameState stays pure state-apply:
//   - "talk to server + route replies": dispatch + the seq-ordered Sequencer.
//   - view-scoped native notifications: the client scans each batch it applies and raises a
//     toast for the view the user is actually looking at.
import { Sequencer } from "./lib/protocol";
import type { CommandBatch, StreamingRouter } from "./lib/protocol";
import type {
  ActionRequest,
  ActorDisplay,
  CommandPayload,
  CommandRecipient,
  ProsecutionPhaseView,
} from "./bindings";
import { slotKeyToString } from "./bindings";
import { actorLabel, GameState, phaseViewEqual, recipientToView } from "./game_state.svelte";
import { UiState } from "./ui_state.svelte";

export const CLIENT_KEY = Symbol("client");

export class ClientState {
  readonly router: StreamingRouter;
  readonly ui = new UiState();
  readonly game = new GameState();
  // Replies to our own requests AND pushed external-action batches funnel through one ordered
  // pipe so they can never race into a desync.
  #seq = new Sequencer();

  constructor(router: StreamingRouter) {
    this.router = router;
    router.onCommands((batch: CommandBatch) =>
      this.#seq.ingest({ seq: batch.seq, run: () => this.#apply(batch.commands) }),
    );
  }

  // Apply a batch to game state and raise any view-scoped toasts for it. Notifications are
  // derived here (not in GameState) so GameState never depends on the router or the UI; they
  // fire BEFORE the apply so a command whose apply consumes state (e.g. KidnapReveal clearing
  // the kidnapping) can still be resolved for the toast text.
  #apply(commands: CommandPayload[]) {
    for (const c of commands) this.#maybe_notify(c);
    this.game.apply_batch(commands);
  }

  // Fire this client's own action. Returns an error string on failure (for UX), or void on
  // success. The reply's state effect (response data + commands) is applied through the same
  // seq-ordered pipe as external batches — never inline — so a reply that arrives ahead of a
  // still-pending external batch waits its turn instead of desyncing. The error, being UX
  // only, is read and returned immediately.
  async dispatch(
    request: ActionRequest,
    args?: Record<string, unknown>,
  ): Promise<string | void> {
    const { seq, execution } = await this.router.sendAction(request);
    const { exec_result } = execution;

    if (exec_result === "Crashed") {
      // A crash carries no commands, but the transport still consumed a seq — feed the
      // Sequencer a no-op so later batches don't stay buffered behind a permanent gap.
      this.#seq.ingest({ seq, run: () => {} });
      return "The engine has crashed.";
    }
    const result = exec_result.Standard;
    if ("Err" in result) {
      // Even on failure the engine returns catchup commands that must still be applied.
      const [error, context] = result.Err;
      this.#seq.ingest({ seq, run: () => this.#apply(context.commands) });
      return String(error);
    }
    const [response, context] = result.Ok;
    this.#seq.ingest({
      seq,
      run: () => {
        this.game.handle_response(response, args);
        this.#apply(context.commands);
      },
    });
  }

  // Raise a native OS/browser toast, but only for the view the user is currently looking at —
  // this client holds every view, so without this gate it would toast for all of them. The
  // selected viewer is `ui.viewer` ("Admin" selects the System view). Best-effort UX: failure
  // is swallowed inside the router.
  notify(recipient: CommandRecipient, title: string, body: string): void {
    const selected = this.ui.viewer === "Admin" ? "System" : this.ui.viewer;
    if (recipientToView(recipient) !== selected) return;
    void this.router.notify({ title, body });
  }

  quit(): void {
    this.router.quit();
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
