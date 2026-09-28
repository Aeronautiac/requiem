// yagami's REST surface, over `host.request`. Every call answers with a Reply, the same shape the
// socket path uses, so a render site handles one kind of failure.
import type {
  AccountPacket,
  GameCreationPacket,
  RosterEntry,
  SavedKey,
} from "./bindings.ts";
import type { Host, HttpResponse, Reply } from "./host.ts";

async function call(
  host: Host,
  method: string,
  path: string,
  body?: unknown,
): Promise<Reply<string>> {
  let response: HttpResponse;
  try {
    response = await host.request(method, path, body);
  } catch {
    return { ok: false, error: { kind: "network" } };
  }
  if (response.status >= 200 && response.status < 300) {
    return { ok: true, value: response.body };
  }
  // yagami refuses with its ServerError variant as a JSON string. Anything else (a proxy's error
  // page, an axum rejection for a malformed body) is shown by its status.
  let code = `http_${response.status}`;
  try {
    const parsed = JSON.parse(response.body);
    if (typeof parsed === "string") code = parsed;
  } catch {
    // not JSON; keep the status code
  }
  return { ok: false, error: { kind: "server", code } };
}

async function call_json<T>(
  host: Host,
  method: string,
  path: string,
  body?: unknown,
): Promise<Reply<T>> {
  const reply = await call(host, method, path, body);
  if (!reply.ok) return reply;
  return { ok: true, value: JSON.parse(reply.value) as T };
}

async function call_empty(
  host: Host,
  method: string,
  path: string,
  body?: unknown,
): Promise<Reply<null>> {
  const reply = await call(host, method, path, body);
  if (!reply.ok) return reply;
  return { ok: true, value: null };
}

export function signup(host: Host, username: string, password: string) {
  return call_empty(host, "POST", "/auth/signup", { username, password });
}

export function login(host: Host, username: string, password: string) {
  return call_empty(host, "POST", "/auth/login", { username, password });
}

export function logout(host: Host) {
  return call_empty(host, "POST", "/auth/logout");
}

export function change_password(host: Host, current: string, next: string) {
  return call_empty(host, "PUT", "/account/password", { current, new: next });
}

// NotLoggedIn is the ordinary answer for a guest, not a failure.
export function account(host: Host) {
  return call_json<AccountPacket>(host, "GET", "/account");
}

// Sends only the key's identity, whatever else the caller's object carries.
export function forget_key(host: Host, saved: SavedKey) {
  return call_empty(host, "DELETE", "/account/keys", { game_id: saved.game_id, key: saved.key });
}

export function roster(host: Host) {
  return call_json<RosterEntry[]>(host, "GET", "/roster");
}

export function create_game(host: Host) {
  return call_json<GameCreationPacket>(host, "POST", "/create_game");
}

export function end_game(host: Host, game_id: number) {
  return call_empty(host, "POST", `/game/${game_id}/end_game`);
}

// Trades a durable key for a single-use ticket; the ticket, not the key, rides the socket URL. A
// logged-in caller also has the key saved to their account.
export function get_ticket(host: Host, game_id: number, key: string) {
  return call(host, "POST", `/game/${game_id}/get_ticket`, { key });
}
