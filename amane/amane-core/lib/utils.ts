// The one place the app decides what a moment looks like, so a message row and a log entry never
// disagree about it.
//
// A timestamp on the wire is GAME time: the sandbox counts up from 0, so a moment is how far into
// the game it happened, not a point on the wall clock (see lawliet lib.rs, "sandboxed time"). The
// raw duration of a game day can change mid-game, but these raw units never do -- so this renders
// the raw elapsed game time with explicit units, and real-world wall time is offered only on hover
// (see wallTimeOf).
export function formatTime(gameMs: number): string {
	const totalSec = Math.max(0, Math.floor(gameMs / 1000));
	const d = Math.floor(totalSec / 86400);
	const h = Math.floor((totalSec % 86400) / 3600);
	const m = Math.floor((totalSec % 3600) / 60);
	const s = totalSec % 60;
	const days = d > 0 ? `${d}d:` : "";
	return `${days}${h}h:${String(m).padStart(2, "0")}m:${String(s).padStart(2, "0")}s`;
}

// The game-clock anchor the client holds: the game's virtual time `time` as of real wall clock
// `sent_at`. Game time advances one-for-one with real time between anchors, so given an anchor this
// turns a GAME timestamp into the real wall-clock moment it happened at -- the "what does that mean
// in my calendar" hover value. Undefined until the server has sent an anchor.
//
// `clock` is a view's game_clock (see GameView).
export function wallTimeOf(
	gameMs: number,
	clock: { time: number; sent_at: number } | null | undefined,
): string | undefined {
	if (!clock) return undefined;
	const wall = clock.sent_at + (gameMs - clock.time);
	const d = new Date(wall);
	// The game can span real days, so a bare time would silently read as "today". Prefix the date
	// whenever the moment is not today; same-day moments stay just the time.
	const date = d.toDateString() === new Date().toDateString()
		? ""
		: `${d.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" })}, `;
	return `${date}${d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
}

// Engine Time is unix ms, so durations like a notebook-write delay arrive in ms. Shows the largest
// one or two non-zero leading units: "2m 30s", "1h 1m", "800ms".
export function formatDuration(ms: number): string {
	if (ms < 1000) return `${ms}ms`;
	const units: [number, string][] = [
		[86400000, "d"],
		[3600000, "h"],
		[60000, "m"],
		[1000, "s"],
	];
	const parts: string[] = [];
	let rem = ms;
	for (const [size, label] of units) {
		if (rem >= size) {
			const val = Math.floor(rem / size);
			parts.push(`${val}${label}`);
			rem -= val * size;
			if (parts.length === 2) break;
		} else if (parts.length > 0) {
			break; // stop at the first gap once we've started, keeping it to leading units
		}
	}
	return parts.join(" ");
}
