// The top of the client. A client shows the PLATFORM, not one game, so this owns the connection
// lifecycle and SessionState owns nothing about it: joining builds a fresh session, leaving throws
// it away. A game's state is deliberately not kept after leaving — the server replays everything a
// connection is entitled to as its first batch, so rejoining reconstructs it, and a stale local
// copy could only be missing whatever happened while away.

import { SessionState } from "./session.svelte";
import type { GameTarget, HostContext } from "./lib/protocol";

export const CLIENT_KEY = Symbol("client");

export type Phase =
  | { status: "idle" }
  | { status: "joining" }
  | { status: "joined" }
  // The reason a join failed or a live game ended, for the platform screen to show.
  | { status: "failed"; reason: string };

export class ClientState {
  readonly host: HostContext;
  phase = $state<Phase>({ status: "idle" });
  // The joined game, or null. The UI switches on this rather than on `phase` so it can never
  // render a game screen without a session behind it.
  session = $state<SessionState | null>(null);
  // Kept for display and for rejoining after a drop.
  target = $state<GameTarget | null>(null);

  constructor(host: HostContext) {
    this.host = host;
  }

  // Called once by the shell. A host with an implicit game goes straight in, so the join screen
  // never appears where it would be an unfillable form.
  start(): void {
    if (this.host.implicitGame) void this.join(this.host.implicitGame);
  }

  get canAdminister(): boolean {
    return this.host.platform !== undefined;
  }

  async join(target: GameTarget): Promise<void> {
    if (this.phase.status === "joining") return; // one attempt at a time
    this.leave();
    this.phase = { status: "joining" };

    try {
      const connection = await this.host.connect(target);
      this.target = target;
      this.session = new SessionState(connection, this.host);
      this.phase = { status: "joined" };
    } catch (e) {
      this.phase = { status: "failed", reason: reason(e) };
    }
  }

  // NOT the same as ending the game: it keeps running on the server without us, and EndGame (a
  // control) is what destroys it.
  leave(): void {
    this.session?.connection.close();
    // Anything still waiting on a reply will never get one now.
    this.session?.abandon();
    this.session = null;
    this.target = null;
    if (this.phase.status === "joined") this.phase = { status: "idle" };
  }

  // The connection died on its own (server gone, key revoked, network). Distinguished from
  // `leave` only by ending up in `failed`, so the platform screen can say why.
  dropped(reason: string): void {
    this.leave();
    this.phase = { status: "failed", reason };
  }

  // The platform screen's failure banner offers a dismiss button. This only clears a `failed`
  // phase; a joined game is left through `leave`, not here.
  dismiss(): void {
    if (this.phase.status === "failed") this.phase = { status: "idle" };
  }
}

function reason(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
