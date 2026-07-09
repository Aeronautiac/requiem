// The router seam. `StreamingRouter` (in protocol.ts) is the canonical interface;
// this alias keeps the historical `Router` name available for imports. It's injected
// into GameState via `attach` — components dispatch through `game.dispatch`, not the
// router directly, so there's no router context key.
export type { StreamingRouter as Router } from "./protocol";
