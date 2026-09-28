// What an output's recipient list names. The server resolved the engine's recipient into one or
// more of these and filtered them against this connection's privileges; the client routes on them.
import type { Output } from "../bindings.ts";
import { slotKeyToString } from "../bindings.ts";

// The viewports this output is addressed to, in recipient order.
export function recipientViewports(out: Output): string[] {
  const viewports: string[] = [];
  for (const recipient of out.recipients) {
    if (typeof recipient !== "string" && "Viewport" in recipient) {
      viewports.push(slotKeyToString(recipient.Viewport));
    }
  }
  return viewports;
}

// The actors this output is addressed to, in recipient order.
export function recipientActors(out: Output): string[] {
  const actors: string[] = [];
  for (const recipient of out.recipients) {
    if (typeof recipient !== "string" && "Player" in recipient) {
      actors.push(slotKeyToString(recipient.Player));
    }
  }
  return actors;
}
