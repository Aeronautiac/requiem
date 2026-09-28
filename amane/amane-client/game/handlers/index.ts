// One function per command, and a table saying which. A command is anything on the wire — an
// engine command or a server command (a log dump, a roster) — normalized to the same shape, so
// there is exactly one path into a handler and no engine/server split.
//
// Every handler receives the view it is being applied INTO, and there is no other tier to write
// to. A handler decides nothing about WHO sees what, only what the fact means once it arrived. It
// decides nothing about notifications either: surfaces observe deliveries and judge that.
import type { WireCommand } from "../../bindings.ts";
import type { View } from "../view.ts";

import { actorHandlers } from "./actors.ts";
import { channelHandlers } from "./channels.ts";
import { feedHandlers } from "./feeds.ts";
import { notebookHandlers } from "./notebooks.ts";
import { orgHandlers } from "./orgs.ts";
import { pollHandlers } from "./polls.ts";
import { prosecutionHandlers } from "./prosecutions.ts";
import { serverHandlers } from "./server.ts";
import { viewportHandlers } from "./viewports.ts";
import { worldHandlers } from "./world.ts";

// What a handler is given besides its payload.
export type CmdCtx = {
  view: View;
  timestamp: number;
  // The viewport this command was addressed to, if any. Handlers record it as where an object's
  // content rides, and use it to tell an org's copy of a command from a player's own.
  viewport: string | undefined;
  // The actor it was addressed to, if any.
  actor: string | undefined;
  // Its position in the log. Only entering a viewport needs it.
  pos: number;
  // Hand this view the part of a viewport's past it has not been given.
  backfill: (viewport: string, until: number) => void;
};

// Every key of the command union, and the payload behind one of them. The union is externally
// tagged, so each member is a single-key object and these two are enough to type the table.
export type CommandName =
  WireCommand extends infer U ? (U extends object ? keyof U : never) : never;
export type PayloadOf<K extends PropertyKey> =
  WireCommand extends infer U ? (U extends Record<K, infer P> ? P : never) : never;

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
  ...serverHandlers,
};

export function commandName(cmd: WireCommand): CommandName {
  return Object.keys(cmd)[0] as CommandName;
}

// Apply one command to one view. A command with no handler is ignored on purpose: the engine emits
// plenty this client has no use for.
//
// The one cast in the pipeline lives here. Above it the table is typed per variant; below it the
// payload has been narrowed by the same key that chose the handler.
export function applyCommand(ctx: CmdCtx, cmd: WireCommand): void {
  const name = commandName(cmd);
  const handler = HANDLERS[name] as ((ctx: CmdCtx, payload: unknown) => void) | undefined;
  if (handler) handler(ctx, (cmd as Record<string, unknown>)[name]);
}
