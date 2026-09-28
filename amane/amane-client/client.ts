// The top of the core. A client shows the platform, not one game: it holds the account, at most
// one joined session, and the change signal every surface reads through.
//
// A game's state is not kept after leaving: the server replays everything a connection is entitled
// to as its first batch, so rejoining rebuilds it, and a stale local copy could only be missing
// whatever happened while away.
import * as api from "./api.ts";
import type { AccountPacket, SavedKey } from "./bindings.ts";
import type { Observer } from "./game/game.ts";
import type { ExecError, GameConnection, Host, Reply } from "./host.ts";
import { Session } from "./session.ts";

export type AccountState =
  | { kind: "loading" }
  | { kind: "guest" }
  | { kind: "member"; packet: AccountPacket }
  // The account couldn't be fetched at all (server down, network).
  | { kind: "unreachable"; error: ExecError };

export type Phase =
  | { status: "idle" }
  | { status: "joining" }
  | { status: "joined" }
  // Joining failed; the platform screen says why.
  | { status: "failed"; error: ExecError }
  // A live game's connection died on its own.
  | { status: "dropped"; reason: string };

export class Client {
  readonly host: Host;
  account: AccountState = { kind: "loading" };
  phase: Phase = { status: "idle" };
  // The joined game, or null. A surface switches on this rather than on `phase`, so it can never
  // render a game without a session behind it.
  session: Session | null = null;

  // Moves once per batch, account refetch, or join/leave transition — never per command. It
  // carries no data: it means "state moved, read again".
  version = 0;
  #listeners = new Set<() => void>();
  #observers: Observer[] = [];

  constructor(host: Host) {
    this.host = host;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  }

  // Hooks around every command delivery, in every game this client joins from now on.
  observe(observer: Observer): () => void {
    this.#observers.push(observer);
    return () => {
      const i = this.#observers.indexOf(observer);
      if (i !== -1) this.#observers.splice(i, 1);
    };
  }

  #changed() {
    this.version++;
    for (const listener of this.#listeners) listener();
  }

  // ---- account ----

  async refresh_account(): Promise<void> {
    const reply = await api.account(this.host);
    if (reply.ok) this.account = { kind: "member", packet: reply.value };
    else if (reply.error.kind === "server" && reply.error.code === "NotLoggedIn") {
      this.account = { kind: "guest" };
    } else this.account = { kind: "unreachable", error: reply.error };
    this.#changed();
  }

  // Every call that can change the account answers first, then refetches it, so the caller's
  // reply and the new account arrive together.
  async #then_refresh<T>(reply: Promise<Reply<T>>): Promise<Reply<T>> {
    const result = await reply;
    await this.refresh_account();
    return result;
  }

  signup(username: string, password: string) {
    return this.#then_refresh(api.signup(this.host, username, password));
  }

  login(username: string, password: string) {
    return this.#then_refresh(api.login(this.host, username, password));
  }

  logout() {
    return this.#then_refresh(api.logout(this.host));
  }

  change_password(current: string, next: string) {
    return this.#then_refresh(api.change_password(this.host, current, next));
  }

  forget_key(saved: SavedKey) {
    return this.#then_refresh(api.forget_key(this.host, saved));
  }

  create_game() {
    return this.#then_refresh(api.create_game(this.host));
  }

  end_game(game_id: number) {
    return this.#then_refresh(api.end_game(this.host, game_id));
  }

  // The directory. Not stored: whoever shows it polls it.
  roster() {
    return api.roster(this.host);
  }

  // ---- games ----

  async join(game_id: number, key: string): Promise<Reply<null>> {
    if (this.phase.status === "joining") return { ok: false, error: { kind: "denied" } };
    this.leave();
    this.phase = { status: "joining" };
    this.#changed();

    const ticket = await api.get_ticket(this.host, game_id, key);
    if (!ticket.ok) return this.#join_failed(ticket.error);

    let connection: GameConnection;
    try {
      connection = await this.host.open_socket(game_id, ticket.value);
    } catch {
      return this.#join_failed({ kind: "network" });
    }

    const session = new Session(game_id, key, connection, this.#observers, () => this.#changed());
    connection.onDropped((reason) => {
      // Only the current session's drop matters; a stale one was already left.
      if (this.session !== session) return;
      this.leave();
      this.phase = { status: "dropped", reason };
      this.#changed();
    });
    this.session = session;
    this.phase = { status: "joined" };
    this.#changed();

    // Joining while logged in saves the key to the account.
    if (this.account.kind === "member") await this.refresh_account();
    return { ok: true, value: null };
  }

  #join_failed(error: ExecError): Reply<null> {
    this.phase = { status: "failed", error };
    this.#changed();
    return { ok: false, error };
  }

  // Not the same as ending the game: it keeps running on the server without us.
  leave() {
    if (!this.session) return;
    this.session.close();
    this.session = null;
    this.phase = { status: "idle" };
    this.#changed();
  }

  // Clears a failed or dropped phase once the platform screen has shown it.
  dismiss() {
    if (this.phase.status !== "failed" && this.phase.status !== "dropped") return;
    this.phase = { status: "idle" };
    this.#changed();
  }
}
