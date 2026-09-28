// The browser host: fetch, WebSocket and Notification. Transport only; the protocol on top lives
// in amane-client.
import type { Batch, ServerInput } from "amane-client/bindings.ts";
import type { GameConnection, Host, HttpResponse, Toast } from "amane-client/host.ts";

export function createWebHost(base_url: string): Host {
  return {
    async request(method: string, path: string, body?: unknown): Promise<HttpResponse> {
      const response = await fetch(`${base_url}${path}`, {
        method,
        // The session cookie. SameSite=Lax, so it only rides same-site requests: the page and
        // the server must share a hostname (ports don't matter).
        credentials: "include",
        headers: body === undefined ? undefined : { "Content-Type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      return { status: response.status, body: await response.text() };
    },

    open_socket(game_id: number, ticket: string): Promise<GameConnection> {
      const url = new URL(`${base_url}/game/${game_id}/ws`);
      url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
      url.searchParams.set("ticket", ticket);
      const socket = new WebSocket(url);

      let on_batch: ((batch: Batch) => void) | undefined;
      let on_dropped: ((reason: string) => void) | undefined;
      // Past `open`, a close means a live game was lost; a deliberate close clears this first.
      let live = false;

      function drop(reason: string) {
        if (!live) return;
        live = false;
        on_dropped?.(reason);
      }

      socket.addEventListener("message", (event) => {
        if (typeof event.data !== "string") return; // the protocol is text-only
        let batch: Batch;
        try {
          batch = JSON.parse(event.data);
        } catch {
          // The two sides disagree about the wire itself. A lost frame may have carried a reply,
          // which would put every later reply on the wrong caller, so this one does disconnect.
          socket.close();
          drop("The server sent something this client could not read.");
          return;
        }
        on_batch?.(batch);
      });
      socket.addEventListener("close", () => drop("Disconnected from the game."));

      const connection: GameConnection = {
        send(input: ServerInput) {
          // Dropped rather than queued when the socket is gone: yagami stamps game time on
          // arrival, so an input flushed after a reconnect would land long after it was meant.
          // The caller hears about it when its waiter is abandoned.
          if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(input));
        },
        onBatch(handler) {
          on_batch = handler;
        },
        onDropped(handler) {
          on_dropped = handler;
        },
        close() {
          live = false;
          socket.close();
        },
      };

      return new Promise((resolve, reject) => {
        socket.addEventListener("open", () => {
          live = true;
          resolve(connection);
        }, { once: true });
        // Before `open`, a close IS the failure to connect.
        socket.addEventListener("close", () => {
          if (!live) reject(new Error("The server closed the connection."));
        }, { once: true });
      });
    },

    async notify({ title, body }: Toast) {
      try {
        if (!("Notification" in globalThis)) return;
        let granted = Notification.permission === "granted";
        if (!granted && Notification.permission !== "denied") {
          granted = (await Notification.requestPermission()) === "granted";
        }
        if (granted) new Notification(title, { body });
      } catch {
        // Best-effort: a refused or missing notification system is not an error.
      }
    },

    // No `quit`: a tab cannot close itself.
  };
}
