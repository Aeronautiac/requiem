// The router, and nothing else.
//
// This holds the views, the one command log, and the rule for which views a command reaches. It
// holds no game state of its own: there is nowhere for a fact to land except inside a view, which
// is what makes it impossible for one view to render something addressed to another.
import { SvelteMap } from "svelte/reactivity";
import { outputCommand } from "../bindings";
import type { ServerOutput } from "../bindings";
import { slotKeyToString } from "../bindings";
import { applyCommand } from "./commands";
import type { WireCommand } from "../bindings";
import { gateActors, gateViewports } from "./helpers.svelte";
import { History } from "./history";
import type { Toast } from "../lib/protocol";
import { GameView } from "./view.svelte";

export const GAME_STATE_KEY = Symbol("game_state");

// Given the view a command landed in and a toast a handler already composed, decide whether the
// human sees it. Injected by the session, which alone knows what is on screen and whether we are
// live — the handler decided the words, this only decides delivery. A no-op until wired, so an
// unwired GameState (a test, a fresh construction) simply raises nothing.
export type NotifySink = (view: GameView, toast: Toast) => void;

// Backfill replays history a late-arriving view missed; none of it is news, so it must never toast.
const NO_NOTIFY: (toast: Toast) => void = () => {};

// One delivery: the view a command reaches, along with the viewport and actor it is about *for
// that view*. A gate list can name several actors and viewports; each named view is delivered
// with its own context, not a single hard-coded first one.
export interface Delivery {
  view: GameView;
  viewport: string | undefined;
  actor: string | undefined;
}

export class GameState {
  // Actor key -> that actor's world. "System" is the admin view: it reads every viewport, holds no
  // actors of its own to enter with, and is the only view the personal-info System copies reach.
  views = new SvelteMap<string, GameView>();

  #history = new History();

  // Where a toast-worthy event goes to be judged. The session installs the real one; until then a
  // handler's notify is inert.
  #notify_sink: NotifySink = () => {};

  constructor() {
    this.views.set("System", new GameView("System"));
  }

  // The session wires this once, right after construction. Kept separate from the constructor so a
  // caller that only needs to fold commands (a test) is not forced to hand one over.
  set_notifier(sink: NotifySink) {
    this.#notify_sink = sink;
  }

  // Reset the SAME instance in place, never a whole new one: the UI binds to this exact object
  // through context, so replacing it would strand every component on the stale pre-resync layers.
  // Everything is cleared and re-seeded with a fresh System view, so the catch-up folds into clean
  // state. Mutating the reactive maps (not reassigning the field) is what keeps components live.
  reset() {
    this.#history = new History();
    this.views.clear();
    this.views.set("System", new GameView("System"));
  }

  // Fold one server output into the wire. Every output is routed by its own view gates and applied
  // through ONE handler table — engine command, log dump, roster: the client does not distinguish,
  // it obeys what it receives. Command ordering within a batch is significant
  // (create-before-reference, last-write-wins perms), so never reorder.
  apply_output(out: ServerOutput): number {
    const cmd = outputCommand(out);
    const pos = this.#history.append(out);
    const deliveries = this.#recipients(out);
    for (const { view, viewport, actor } of deliveries) {
      this.#deliver(view, cmd, out.time, actor, viewport, pos, (toast) =>
        this.#notify_sink(view, toast),
      );
    }
    return deliveries.length;
  }

  // Deliver one command into one view, live or replayed. Everything a handler is allowed to touch
  // reaches it through here.
  #deliver(
    view: GameView,
    cmd: WireCommand,
    time: number,
    actor: string | undefined,
    viewport: string | undefined,
    pos: number,
    notify: (toast: Toast) => void,
  ) {
    // So a later entry by another of this client's actors knows where its own gap begins.
    if (viewport !== undefined) view.deliver_to(viewport, pos + 1);

    applyCommand(
      {
        view,
        timestamp: time,
        viewport,
        actor,
        pos,
        notify,
        backfill: (v, until) => this.#backfill(view, v, until),
      },
      cmd,
    );
  }

  // Which views an output lands in, and the context each is delivered with, decided from its view
  // gates. The server already decided this CONNECTION may see the output; the gates say who within
  // the connection it concerns and what it is about for each of them.
  #recipients(out: ServerOutput): Delivery[] {
    const result: Delivery[] = [];
    const seen = new Set<GameView>();
    const command_viewports = gateViewports(out);
    const command_actors = gateActors(out);

    for (const gate of out.view_gates) {
      if (gate === "Admin") {
        // System has no viewport or actor of its own — it reads everything. For a command that
        // also names a viewport (every viewport command carries an Admin gate), the viewport the
        // content rides is still what a handler needs to record where objects live.
        const view = this.system_view();
        if (!seen.has(view)) {
          seen.add(view);
          result.push({ view, viewport: command_viewports[0], actor: command_actors[0] });
        }
      } else if (typeof gate !== "string" && "Player" in gate) {
        const key = slotKeyToString(gate.Player);
        const view = this.view_for(key);
        if (!seen.has(view)) {
          seen.add(view);
          result.push({ view, viewport: command_viewports[0], actor: key });
        }
      } else if (typeof gate !== "string" && "Viewport" in gate) {
        const viewport = slotKeyToString(gate.Viewport);
        for (const [key, view] of this.views) {
          // System reads every viewport — the server sends it everything (an Admin gate rides
          // alongside every Viewport gate), and it holds no actors of its own to enter with. That
          // is what lets admin watch a deception: they see the fiction through the same viewport
          // the players do.
          if (key !== "System" && view.viewports.has(viewport)) {
            if (!seen.has(view)) {
              seen.add(view);
              result.push({ view, viewport, actor: command_actors[0] });
            }
          }
        }
      }
      // "Connection" routes to no view: the output is this connection's own fact (its privileges),
      // which the session reads directly rather than displaying.
    }
    return result;
  }

  // Hand one view the part of a viewport's past it has not been given.
  //
  // The server backfills a viewport once per CONNECTION — only its first holder — which is correct
  // for a connection and insufficient here, where state is per-actor. A key holding several actors
  // (an admin key holds every actor) has already been sent viewports a view entering now never
  // saw, and no second backfill is coming.
  //
  // The two are complementary and cannot overlap: the server sends what the connection lacked,
  // this replays what the connection had and this view lacked. The watermark separates them.
  #backfill(view: GameView, viewport: string, until: number) {
    for (const [pos, out] of this.#history.range(viewport, view.delivered(viewport), until)) {
      this.#deliver(view, outputCommand(out), out.time, gateActors(out)[0], viewport, pos, NO_NOTIFY);
    }
    view.deliver_to(viewport, until);
  }

  // The view for an actor, created on demand.
  //
  // Views cannot wait for AddPlayer's *response*: an action's commands are applied before its
  // response is settled, and a player's own creation batch is already full of commands addressed
  // to them. Creating the view lazily on first sight is what makes that batch land instead of
  // being dropped.
  view_for(key: string): GameView {
    let view = this.views.get(key);
    if (!view) {
      view = new GameView(key);
      this.views.set(key, view);
    }
    return view;
  }

  system_view(): GameView {
    return this.views.get("System")!;
  }

  // The view a viewer key selects. "Admin" is the UI's name for System.
  //
  // Created on demand rather than returned as possibly-absent, because an empty view is the honest
  // answer for an actor nothing has arrived for yet: it knows nothing, and renders as knowing
  // nothing. Handing components an optional here would make every render site carry a fallback for
  // a case that already has a correct representation.
  view_of(viewer: string): GameView {
    return this.view_for(viewer === "Admin" ? "System" : viewer);
  }
}
