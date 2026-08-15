import type { Output } from "../bindings";
import { gateViewports } from "./helpers.svelte";

// Every command this client has received, in order, plus an index over the viewport-addressed
// ones. Deliberately the same structure as yagami's History — the server holds exactly this and
// filters it per connection, and the client's problem is the same one at a finer granularity.
//
// The one thing shared by every view. Not reactive: nothing renders from the log, only from the
// state applying it produces.
export class History {
  #log: Output[] = [];
  // viewport key -> its positions in #log, ascending. Positions, not payloads: there is exactly
  // one copy of every command.
  #index = new Map<string, number[]>();

  // Returns the position, which is what watermarks are measured in.
  //
  // Everything that travels the per-command wire enters the log, engine commands and sim/server
  // commands alike — a log dump is delivered through the same per-view path as a command, so it
  // has to be replayable the same way.
  append(out: Output): number {
    const pos = this.#log.length;
    this.#log.push(out);
    for (const viewport of gateViewports(out)) {
      let positions = this.#index.get(viewport);
      if (!positions) {
        positions = [];
        this.#index.set(viewport, positions);
      }
      positions.push(pos);
    }
    return pos;
  }

  // Everything addressed to `viewport` in [from, until), with each command's position.
  //
  // Note what CANNOT come back from here: EnterViewport and ExitViewport are addressed to the
  // actor they concern, never to a viewport, so replaying a viewport's history can never contain
  // another access change. That is what keeps backfill from recursing.
  *range(viewport: string, from: number, until: number): Generator<[number, Output]> {
    for (const pos of this.#index.get(viewport) ?? []) {
      if (pos < from) continue;
      if (pos >= until) return;
      yield [pos, this.#log[pos]];
    }
  }
}
