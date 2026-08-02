// One function per command, and a table saying which.
//
// Every handler receives the view it is being applied INTO, and there is no other tier to write
// to: if a fact should reach a view, the router delivered the command to that view and the handler
// writes it there. A handler therefore decides nothing about WHO sees what — only what the fact
// means once it has arrived.
import type { Command, CommandRecipient } from "../../bindings";
import type { Toast } from "../../lib/protocol";
import type { GameView } from "../view.svelte";

import { actorHandlers } from "./actors";
import { channelHandlers } from "./channels";
import { feedHandlers } from "./feeds";
import { notebookHandlers } from "./notebooks";
import { orgHandlers } from "./orgs";
import { pollHandlers } from "./polls";
import { prosecutionHandlers } from "./prosecutions";
import { viewportHandlers } from "./viewports";
import { worldHandlers } from "./world";

// What a handler is given besides its payload.
export type CmdCtx = {
  view: GameView;
  recipient: CommandRecipient;
  timestamp: number;
  // The viewport this command was addressed to, if any. Handlers use it to record where an
  // object's content rides, and to tell an org's copy of a command from a player's own.
  viewport: string | undefined;
  // The actor it was addressed to, if any.
  actor: string | undefined;
  // Its position in the log. Only entering a viewport needs it.
  pos: number;
  // Raise a toast — separate from writing the event into the view, which the handler does itself.
  // This is only the effect. The handler is the one place with the recipient, content and channel
  // in hand, so it composes the words; a handler with nothing to say simply doesn't call this.
  // Inert during backfill; the session still decides whether the view it landed in is on screen.
  notify: (toast: Toast) => void;
  // Hand this view the part of a viewport's past it has not been given.
  backfill: (viewport: string, until: number) => void;
};

// Every key of the Command union, and the payload behind one of them. The union is externally
// tagged, so each member is a single-key object and these two are enough to type the table.
export type CommandName = Command extends infer U ? (U extends object ? keyof U : never) : never;
export type PayloadOf<K extends PropertyKey> =
  Command extends infer U ? (U extends Record<K, infer P> ? P : never) : never;

export type Handlers = {
  [K in CommandName]?: (ctx: CmdCtx, payload: PayloadOf<K>) => void;
};

const HANDLERS: Handlers = {
  ...viewportHandlers,
  ...channelHandlers,
  ...notebookHandlers,
  ...feedHandlers,
  ...actorHandlers,
  ...orgHandlers,
  ...pollHandlers,
  ...prosecutionHandlers,
  ...worldHandlers,
};

// Apply one command to one view. A command with no handler is ignored on purpose: the engine emits
// plenty this client has no use for, and the alternative is a stub per variant.
//
// The one cast in the pipeline lives here. Above it the table is fully typed per variant; below it
// the payload has been narrowed by the same key that chose the handler.
export function applyCommand(ctx: CmdCtx, cmd: Command): void {
  const name = Object.keys(cmd)[0] as CommandName;
  const handler = HANDLERS[name] as ((ctx: CmdCtx, payload: unknown) => void) | undefined;
  if (!handler) return;
  handler(ctx, (cmd as Record<string, unknown>)[name as string]);
}
