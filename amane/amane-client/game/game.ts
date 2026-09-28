// One game's state as this connection has been told it: the views, the one command log, and the
// rule for which views a command reaches.
//
// Thrown away, never reset. An Initialize batch builds a new Game, so nothing is ever cleared in
// place and nothing can survive a resync by accident.
import type { Output, WireCommand } from "../bindings.ts";
import { outputCommand, slotKeyToString } from "../bindings.ts";
import { applyCommand, commandName } from "./handlers/index.ts";
import type { CommandName } from "./handlers/index.ts";
import { History } from "./history.ts";
import { recipientActors, recipientViewports } from "./recipients.ts";
import { View } from "./view.ts";

// One command, as it lands in one view.
export type Delivered = {
  view: string; // the view's own key ("System" for admin)
  name: CommandName;
  command: WireCommand;
  time: number;
  // The viewport it rode, if any. Tells an org's copy of a command from a player's own.
  viewport: string | undefined;
  // Catch-up or backfill: history, not news. Reported, not acted on; muting history is the
  // observer's choice.
  replayed: boolean;
};

// Hooks around every delivery, live or replayed. `before` sees the view before the fold and
// `after` sees it after; whatever `before` returned is handed straight to `after`, through the call
// stack rather than a field, because a delivery can nest inside another (EnterViewport backfills
// from inside its own handler).
//
// The core is mutable, so `before` must copy VALUES out of the view (a boolean, a phase), never
// hold a reference: an object read in `before` already holds the new state by the time `after`
// runs. Hooks run per command, often mid-batch, so they decide rather than render, and must be
// cheap: they fire for every catch-up command too.
export type Observer = {
  before?(view: View, delivered: Delivered): unknown;
  after(view: View, delivered: Delivered, memo: unknown): void;
};

// A delivery whose handler threw. The command is skipped in that one view and everything else goes
// on, because the same person receives the same stream on every reconnect: stopping would lock
// them out until the client is fixed. What they see may be wrong, and this is how a surface says
// so.
export type Fault = {
  view: string | null; // null when routing itself failed, before any view was chosen
  name: CommandName;
  time: number;
  error: string;
};

// One delivery target: the view, and the viewport and actor the command is about for that view.
type Delivery = { view: View; viewport: string | undefined; actor: string | undefined };

export class Game {
  // View key -> that view's world. An actor's key for an actor, "System" for what is addressed to
  // Admin.
  views = new Map<string, View>([["System", new View("System")]]);
  faults: Fault[] = [];

  #history = new History();
  // The client's list, shared by reference so an observer added later reaches this game too.
  #observers: Observer[];

  constructor(observers: Observer[]) {
    this.#observers = observers;
  }

  // Fold one server output. Command ordering within a batch is significant (create before
  // reference, last write wins), so never reorder.
  apply_output(out: Output, replayed: boolean) {
    const command = outputCommand(out);
    let pos: number;
    let deliveries: Delivery[];
    try {
      pos = this.#history.append(out);
      deliveries = this.#recipients(out);
    } catch (error) {
      this.#fault(null, command, out.time, error);
      return;
    }
    for (const { view, viewport, actor } of deliveries) {
      this.#deliver(view, command, out.time, actor, viewport, pos, replayed);
    }
  }

  // Deliver one command into one view. Everything a handler may touch reaches it through here.
  #deliver(
    view: View,
    command: WireCommand,
    time: number,
    actor: string | undefined,
    viewport: string | undefined,
    pos: number,
    replayed: boolean,
  ) {
    // So a later entry by another of this client's actors knows where its own gap begins.
    if (viewport !== undefined) view.deliver_to(viewport, pos + 1);

    const delivered: Delivered = {
      view: view.own_key,
      name: commandName(command),
      command,
      time,
      viewport,
      replayed,
    };
    // A snapshot, so an observer removed mid-delivery doesn't shift the memos out of line.
    const observers = this.#observers.slice();
    const memos = observers.map((observer) => {
      try {
        return observer.before?.(view, delivered);
      } catch (error) {
        console.error("[amane] observer failed before", delivered.name, error);
        return undefined;
      }
    });

    try {
      applyCommand(
        {
          view,
          timestamp: time,
          viewport,
          actor,
          pos,
          backfill: (v, until) => this.#backfill(view, v, until),
        },
        command,
      );
    } catch (error) {
      this.#fault(view.own_key, command, time, error);
      // The fold didn't finish, so there is no "after" to compare against.
      return;
    }

    observers.forEach((observer, i) => {
      try {
        observer.after(view, delivered, memos[i]);
      } catch (error) {
        // A lost toast, not wrong state: logged, not a fault.
        console.error("[amane] observer failed after", delivered.name, error);
      }
    });
  }

  #fault(view: string | null, command: WireCommand, time: number, error: unknown) {
    const name = commandName(command);
    console.error(
      `[amane] failed to apply ${name} to view ${view ?? "(routing)"} at ${time}:`,
      error,
      command,
    );
    this.faults.push({ view, name, time, error: String(error) });
  }

  // Which views an output lands in, decided from its recipients and nothing else. The server
  // already decided this CONNECTION may see the output; the recipients say who within the
  // connection it is for:
  //   - Admin:    the System view.
  //   - Player:   that actor's view.
  //   - Viewport: every view currently holding the viewport.
  // `Log` never reaches a client, and a connection-level output has no recipients at all: the
  // session reads it directly.
  //
  // The context each view gets is the viewport the command rides (for recording where an object
  // lives) and the actor it names, if any.
  #recipients(out: Output): Delivery[] {
    const result: Delivery[] = [];
    const seen = new Set<View>();
    const viewports = recipientViewports(out);
    const actors = recipientActors(out);

    for (const recipient of out.recipients) {
      if (recipient === "Admin") {
        const view = this.system_view();
        if (!seen.has(view)) {
          seen.add(view);
          result.push({ view, viewport: viewports[0], actor: actors[0] });
        }
      } else if (typeof recipient !== "string" && "Player" in recipient) {
        const key = slotKeyToString(recipient.Player);
        const view = this.view_for(key);
        if (!seen.has(view)) {
          seen.add(view);
          result.push({ view, viewport: viewports[0], actor: key });
        }
      } else if (typeof recipient !== "string" && "Viewport" in recipient) {
        const viewport = slotKeyToString(recipient.Viewport);
        for (const view of this.views.values()) {
          if (view.viewports.has(viewport) && !seen.has(view)) {
            seen.add(view);
            result.push({ view, viewport, actor: actors[0] });
          }
        }
      }
    }
    return result;
  }

  // Hand one view the part of a viewport's past it has not been given.
  //
  // The server backfills a viewport once per CONNECTION, which is insufficient here, where state
  // is per-actor: a key holding several actors has already been sent viewports a view entering
  // now never saw. The two cannot overlap: the server sends what the connection lacked, this
  // replays what the connection had and this view lacked. The watermark separates them.
  #backfill(view: View, viewport: string, until: number) {
    for (const [pos, out] of this.#history.range(viewport, view.delivered(viewport), until)) {
      this.#deliver(view, outputCommand(out), out.time, recipientActors(out)[0], viewport, pos, true);
    }
    view.deliver_to(viewport, until);
  }

  // The view for an actor, created on first sight. Views can't wait for AddPlayer's response: a
  // player's own creation batch is already full of commands addressed to them.
  view_for(key: string): View {
    let view = this.views.get(key);
    if (!view) {
      view = new View(key);
      this.views.set(key, view);
    }
    return view;
  }

  system_view(): View {
    return this.views.get("System")!;
  }
}
