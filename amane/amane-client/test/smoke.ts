// Folds a hand-built command stream through a real Session with no browser and no server: the check
// that the core is headless. Run with `node test/smoke.ts` (Node strips the types itself).
import assert from "node:assert/strict";
import type { Batch, Output, ServerInput } from "../bindings.ts";
import type { Delivered } from "../game/game.ts";
import type { GameConnection } from "../host.ts";
import { viewers } from "../queries/session.ts";
import { Session } from "../session.ts";

// A connection whose batches this script pushes by hand.
let push: (batch: Batch) => void = () => {};
const sent: ServerInput[] = [];
const connection: GameConnection = {
  send: (input) => sent.push(input),
  onBatch: (handler) => {
    push = handler;
  },
  onDropped: () => {},
  close: () => {},
};

const deliveries: Delivered[] = [];
let changes = 0;
const session = new Session(
  1,
  "key",
  connection,
  [{ after: (_view, delivered) => deliveries.push(delivered) }],
  () => changes++,
);

const player = { idx: 1, version: 0 };
const viewport = { idx: 7, version: 0 };
const channel = { idx: 3, version: 0 };
const on_viewport = (data: Output["data"], time = 10): Output => ({
  time,
  recipients: [{ Viewport: viewport }, "Admin"],
  data,
});

// A channel is created on a viewport before anyone holds it, then a player enters, then speaks.
push({
  kind: "Initialize",
  outputs: [
    { time: 0, recipients: [], data: { Server: { Privileges: { actors: "All", capabilities: ["Administer"] } } } },
    on_viewport({ Engine: { MapChannel: { channel_id: channel, kind: { World: "General" } } } }),
    on_viewport({ Engine: { SetChannelLoggable: { channel_id: channel, loggable: true } } }),
    on_viewport({ Engine: { AddMessage: { channel_id: channel, content: "before", sender_display: "System" } } }),
    { time: 20, recipients: [{ Player: player }], data: { Engine: { EnterViewport: { viewport, actor: player } } } },
    // A handler that throws must not stop the batch: ProfileRoster with a non-array payload.
    { time: 21, recipients: ["Admin"], data: { Sim: { ProfileRoster: { profiles: null as never } } } },
  ],
});
push({
  kind: { Live: null },
  outputs: [on_viewport({ Engine: { AddMessage: { channel_id: channel, content: "after", sender_display: "System" } } }, 30)],
});

const game = session.game;
const key = "3:0";

// System got everything addressed to Admin; the player got the backlog on entry, then live.
for (const view_key of ["System", "1:0"]) {
  const ch = game.views.get(view_key)?.channels.get(key);
  assert.ok(ch, `${view_key} holds the channel`);
  assert.equal(ch.loggable, true);
  assert.deepEqual(
    ch.events.map((e) => ("Message" in e.data ? e.data.Message.content : null)),
    ["before", "after"],
  );
}
assert.equal(game.views.get("1:0")!.frozen("7:0"), false);

// The backfill is reported as replayed; the live message is not.
const player_messages = deliveries.filter((d) => d.view === "1:0" && d.name === "AddMessage");
assert.deepEqual(player_messages.map((d) => d.replayed), [true, false]);

// The bad command became a fault, not a crash.
assert.equal(game.faults.length, 1);
assert.equal(game.faults[0].name, "ProfileRoster");

assert.deepEqual(viewers(session), ["System", "1:0"]);
assert.equal(changes, 2);

// A resync builds a new Game rather than clearing the old one.
push({ kind: "Initialize", outputs: [] });
assert.notEqual(session.game, game);
assert.equal(session.game.faults.length, 0);

console.log("smoke ok");
