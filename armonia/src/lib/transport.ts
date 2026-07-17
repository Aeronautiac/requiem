// Tauri implementation of the router seam. On web, replace this file with one that
// talks to your server (WebSocket, fetch, etc.) — the interface is identical.
//
// The server doesn't stamp sequence numbers or push external-action commands yet, so:
//   - seq is synthesised locally. With one initiator and one delivery path, seqs are
//     contiguous and never gap, so the Sequencer applies every reply immediately —
//     behaviour identical to before the streaming refactor.
//   - onCommands has nothing to deliver; it's a no-op until the push channel exists.
// When the server gains ticking + a push channel, swap the seq source to the server's
// and wire onCommands to the push subscription. Nothing on the client side changes.
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { sendAction as rawSendAction } from "../bindings";
import type { ActionRequest } from "../bindings";
import type { CommandBatch, RequestReply, StreamingRouter, Toast } from "./protocol";

export function createTransport(): StreamingRouter {
  let seq = 0;
  return {
    async sendAction(action: ActionRequest): Promise<RequestReply> {
      const execution = await rawSendAction(action);
      return { seq: ++seq, execution };
    },
    onCommands(_handler: (batch: CommandBatch) => void): () => void {
      return () => {};
    },
    async notify({ title, body }: Toast): Promise<void> {
      // Permission is checked lazily on first use (the OS caches the grant), so nothing
      // has to run at startup. Any failure — denied prompt, unavailable service — is
      // swallowed: a missing toast must never break state application.
      try {
        let granted = await isPermissionGranted();
        if (!granted) granted = (await requestPermission()) === "granted";
        if (granted) sendNotification({ title, body });
      } catch {
        // no-op: notifications are best-effort UX
      }
    },
    quit() {
      getCurrentWindow().close();
    },
  };
}
