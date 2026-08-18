// The real host: yagami over REST + websocket. Every production client uses this.
//
// Joining is a two-step handshake:
//   1. POST /game/{id}/get_ticket with the durable key -> a single-use ticket.
//   2. open the socket with that ticket.
// The key never rides the socket URL, where it would leak into logs and proxies; the ticket is
// consumed by the upgrade and authorizes exactly one connection. Because the claim happens in the
// upgrade itself, a 101 is authoritative: past it every failure is a transport failure and never
// an authorization one. Hence the retry rule — a 4xx from step 1 means the credential is bad and
// retrying is pointless, a dead socket means reconnect with a fresh ticket and catch-up replay.

import type {
  GameConnection,
  GameTarget,
  HostContext,
  PlatformApi,
  Toast,
} from "./protocol";
import type { Batch, ServerInput } from "../bindings";

export interface WebHostConfig {
  // Origin of the yagami API, e.g. "https://play.example.com".
  baseUrl: string;
  // Called when a live connection dies on its own, so the platform layer can drop back to the
  // platform screen with a reason and fail anything still awaiting a reply.
  onDropped?: (reason: string) => void;
}

class HttpError extends Error {
  readonly status: number;
  constructor(status: number, body: string) {
    super(body || `request failed (${status})`);
    this.status = status;
  }
}

async function postJson(url: string, body: unknown): Promise<string> {
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const text = await res.text();
  if (!res.ok) throw new HttpError(res.status, text);
  return text;
}

async function getJson(url: string): Promise<unknown> {
  const res = await fetch(url);
  const text = await res.text();
  if (!res.ok) throw new HttpError(res.status, text);
  return JSON.parse(text);
}

function socketUrl(baseUrl: string, gameId: number, ticket: string): string {
  const url = new URL(`${baseUrl}/game/${gameId}/ws`);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.searchParams.set("ticket", ticket);
  return url.toString();
}

export function createWebHost(config: WebHostConfig): HostContext {
  const platform: PlatformApi = {
    async createGame(platformKey: string) {
      const body = await postJson(`${config.baseUrl}/create_game`, {
        platform_key: platformKey,
      });
      return JSON.parse(body);
    },
    async endGame(gameId: number, platformKey: string) {
      await postJson(`${config.baseUrl}/game/${gameId}/end_game`, {
        platform_key: platformKey,
      });
    },
    async roster() {
      const body = await getJson(`${config.baseUrl}/roster`);
      return body as { game_id: number; connections: number }[];
    },
  };

  return {
    platform,
    canQuit: false,

    async connect({ gameId, key }: GameTarget): Promise<GameConnection> {
      // A refusal here is final: the key is wrong, or it has spent its allowance of tickets.
      const ticket = await postJson(
        `${config.baseUrl}/game/${gameId}/get_ticket`,
        { key },
      );

      const socket = new WebSocket(socketUrl(config.baseUrl, gameId, ticket));
      let handler: ((batch: Batch) => void) | undefined;
      let live = false;

      await new Promise<void>((resolve, reject) => {
        socket.addEventListener("open", () => {
          live = true;
          resolve();
        }, { once: true });
        // Before the socket is up a close IS the failure to connect; after, the same event means
        // we lost a live game. Hence the flag.
        socket.addEventListener(
          "close",
          () => {
            if (!live) reject(new Error("The server closed the connection."));
          },
          { once: true },
        );
      });

      socket.addEventListener("message", (event) => {
        if (typeof event.data !== "string") return; // the protocol is text-only
        let batch: Batch;
        try {
          batch = JSON.parse(event.data);
        } catch {
          // The two sides disagree about the wire format. Applying half of it is worse than
          // stopping.
          socket.close();
          config.onDropped?.("The server sent something this client could not read.");
          return;
        }
        handler?.(batch);
      });

      socket.addEventListener("close", () => {
        if (live) {
          live = false;
          config.onDropped?.("Disconnected from the game.");
        }
      });

      return {
        send(input: ServerInput): void {
          // Dropped rather than queued when the socket is gone. yagami stamps game time on
          // ARRIVAL, so an input held here and flushed after a reconnect would land long after
          // the player meant it — silently wrong is worse than lost, and the caller is told
          // either way when its waiter is abandoned.
          if (socket.readyState !== WebSocket.OPEN) return;
          socket.send(JSON.stringify(input));
        },

        onBatch(next: (batch: Batch) => void): () => void {
          handler = next;
          return () => {
            handler = undefined;
          };
        },

        close(): void {
          live = false; // a deliberate close is not a drop; don't report it as one
          socket.close();
        },
      };
    },

    async notify({ title, body }: Toast): Promise<void> {
      // Best-effort: a denied or unavailable permission must never break state application.
      try {
        if (!("Notification" in globalThis)) return;
        let granted = Notification.permission === "granted";
        if (!granted && Notification.permission !== "denied") {
          granted = (await Notification.requestPermission()) === "granted";
        }
        if (granted) new Notification(title, { body });
      } catch {
        // no-op
      }
    },

    // A tab cannot close itself; `canQuit` above is what keeps the UI from offering it.
    quit() { },
  };
}
