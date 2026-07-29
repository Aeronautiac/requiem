// The router, and nothing else.
//
// This holds the views, the one command log, and the rule for which views a command reaches. It
// holds no game state of its own: there is nowhere for a fact to land except inside a view, which
// is what makes it impossible for one view to render something addressed to another.
import { SvelteMap } from "svelte/reactivity";
import type { CommandPayload, CommandRecipient, ProfileUpdate } from "../bindings";
import { slotKeyToString } from "../bindings";
import { applyCommand } from "./commands";
import { recipientToActor, recipientToView, recipientToViewport } from "./helpers.svelte";
import { History } from "./history";
import { GameView } from "./view.svelte";

export const GAME_STATE_KEY = Symbol("game_state");

export class GameState {
  // Actor key -> that actor's world. "System" is the admin view: it reads every viewport, holds no
  // actors of its own to enter with, and is the only view the personal-info System copies reach.
  views = new SvelteMap<string, GameView>();

  #history = new History();

  // slot -> the name the SERVER gave it, for every slot this connection has been told about.
  //
  // Connection-level rather than per-view, because that is what the server sends: one profile
  // channel, gated on the connection having received the MapActor. Views take names from here
  // rather than holding the only copy, so a name that arrived before a view existed is not lost
  // to it. See #name_players.
  #profiles = new SvelteMap<string, string | null>();

  constructor() {
    this.views.set("System", new GameView());
  }

  // The public seam the Sequencer drives. Command ordering within a batch is significant
  // (create-before-reference, last-write-wins perms), so never reorder.
  apply_batch(commands: CommandPayload[]) {
    for (const payload of commands) this.#apply(payload);
    // A batch can introduce a slot, or a whole view, that a profile already arrived for.
    this.#name_players();
  }

  #apply(payload: CommandPayload) {
    const pos = this.#history.append(payload);
    for (const view of this.#recipients(payload.recipient)) {
      this.#deliver(view, payload, pos);
    }
  }

  // Apply one command into one view, live or replayed. Everything a handler is allowed to touch
  // reaches it through here.
  #deliver(view: GameView, { recipient, cmd, timestamp }: CommandPayload, pos: number) {
    const viewport = recipientToViewport(recipient);
    // So a later entry by another of this client's actors knows where its own gap begins.
    if (viewport !== undefined) view.deliver_to(viewport, pos + 1);

    applyCommand(
      {
        view,
        recipient,
        timestamp,
        viewport,
        actor: recipientToActor(recipient),
        pos,
        backfill: (v, until) => this.#backfill(view, v, until),
      },
      cmd,
    );
  }

  // Which views a command lands in.
  //
  // The server already decided this CONNECTION may see it. An Actor-addressed command names
  // exactly one view; a Viewport-addressed one names none — the server sent it because SOME actor
  // here has access, and which ones is a question only the client can answer.
  #recipients(recipient: CommandRecipient): GameView[] {
    const viewport = recipientToViewport(recipient);
    if (viewport === undefined) {
      const key = recipientToView(recipient);
      return key === undefined ? [] : [this.view_for(key)];
    }

    const out: GameView[] = [];
    for (const [key, view] of this.views) {
      // System reads every viewport — the server sends it everything, and it holds no actors of
      // its own to enter with. That is what lets admin watch a deception: they see the fiction
      // through the same viewport the players do, and any truth the engine exposes arrives
      // separately as a System-addressed command to compose against it.
      if (key === "System" || view.viewports.has(viewport)) out.push(view);
    }
    return out;
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
    for (const [pos, payload] of this.#history.range(viewport, view.delivered(viewport), until)) {
      this.#deliver(view, payload, pos);
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
      view = new GameView();
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

  // Does the named view receive a command addressed this way? For callers that already know which
  // view they care about, such as deciding whether to toast for the one on screen.
  view_receives(recipient: CommandRecipient, view_key: string): boolean {
    const viewport = recipientToViewport(recipient);
    if (viewport !== undefined) {
      if (view_key === "System") return true; // see #recipients
      return this.views.get(view_key)?.viewports.has(viewport) ?? false;
    }
    return recipientToView(recipient) === view_key;
  }

  // What the SERVER knows about who occupies a slot, on its own channel beside the command stream.
  //
  // Applied per view, and only where that view already knows the slot. The server gates these on
  // the connection having received the MapActor, but a connection holding several actors is not
  // the same as any one of them having received it — so the check is repeated here, per view,
  // where the answer is actually per-actor.
  apply_profiles(update: ProfileUpdate) {
    for (const [id, profile] of update.profiles) {
      this.#profiles.set(slotKeyToString(id), profile.display_name);
    }
    this.#name_players();
  }

  // Push the current name onto every slot a view holds one for.
  //
  // Existence and naming arrive on different channels with different lifetimes, and only existence
  // is replayable: a view created later backfills the presence viewport and learns the slot, but
  // nothing re-sends the profile that named it. Reconciling after every batch is what keeps a slot
  // from rendering as `player-3v0` forever just because it was named before the view existed.
  //
  // Assigns rather than fills, so a rename lands everywhere the old name was — including clearing
  // one back to null. `#profiles` is the authority; nothing else writes display_name. A slot with
  // no profile at all is left alone.
  //
  // This leaks nothing. A view is only ever named for a slot it already holds, so what it may SEE
  // is still decided entirely by what it was delivered; this only decides what to CALL it.
  #name_players() {
    for (const view of this.views.values()) {
      for (const [key, player] of view.players) {
        const name = this.#profiles.get(key);
        if (name !== undefined) player.display_name = name;
      }
    }
  }
}
