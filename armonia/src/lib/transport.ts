// Tauri implementation of Router. On web, replace this file with one that talks
// to your server (WebSocket, fetch, etc.) — the interface is identical.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { sendAction } from "../bindings";
import type { Router } from "./router";

export function createTransport(): Router {
  return {
    sendAction,
    quit() {
      getCurrentWindow().close();
    },
  };
}
