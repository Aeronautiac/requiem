// The top of the client. A client displays the PLATFORM, not one game: it starts with
// nothing joined, and a game's UI only exists once a connection to that game is live.
//
// So this owns the connection lifecycle and ClientState owns nothing about it. Joining
// builds a fresh ClientState; leaving throws it away. There is deliberately no attempt to
// keep a game's state around after leaving — the server replays everything a connection is
// entitled to as its first batch, so rejoining reconstructs it, and a stale local copy could
// only be wrong (it would be missing everything that happened while away).

import { ClientState } from "./client.svelte";
import type { GameTarget, HostContext } from "./lib/protocol";

export const PLATFORM_KEY = Symbol("platform");

export type Phase =
  | { status: "idle" }
  | { status: "joining" }
  | { status: "joined" }
  // The reason a join failed or a live game ended, for the platform screen to show.
  | { status: "failed"; reason: string };

export class PlatformState {
  readonly host: HostContext;
  phase = $state<Phase>({ status: "idle" });
  // The live game, or null when nothing is joined. The UI switches on this rather than on
  // `phase` so it can never render a game screen without a client behind it.
  client = $state<ClientState | null>(null);
  // Which game is joined, kept for display and for rejoining after a drop.
  target = $state<GameTarget | null>(null);

  constructor(host: HostContext) {
    this.host = host;
  }

  // Called once by the shell. A host with an implicit game goes straight in, so the join
  // screen never appears for a host where it would be an unfillable form.
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
      this.client = new ClientState(connection, this.host);
      this.phase = { status: "joined" };
    } catch (e) {
      this.phase = { status: "failed", reason: reason(e) };
    }
  }

  // Tear down the local side of a game. Note this is NOT the same as ending the game: the
  // game keeps running on the server without us, and EndGame (a control) is what destroys it.
  leave(): void {
    this.client?.connection.close();
    // Anything still waiting on a reply will never get one now.
    this.client?.abandon("Left the game.");
    this.client = null;
    this.target = null;
    if (this.phase.status === "joined") this.phase = { status: "idle" };
  }

  // The connection died on its own (server gone, key revoked, network). Distinguished from
  // `leave` only by ending up in `failed`, so the platform screen can say why.
  dropped(reason: string): void {
    this.leave();
    this.phase = { status: "failed", reason };
  }
}

function reason(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
