// Tauri implementation of Router. On web, replace this file with one that talks
// to your server (WebSocket, fetch, etc.) — the interface is identical.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { commands } from "../bindings";
import type { Router } from "./router";

function unwrap<T>(result: { status: "ok"; data: T } | { status: "error"; error: string }): T {
  if (result.status === "error") throw new Error(result.error);
  return result.data;
}

export function createTransport(): Router {
  return {
    async sendAction(action) {
      const res = unwrap(await commands.sendAction(action));
      return res;
    },
    quit() {
      getCurrentWindow().close();
    },
  };
}
