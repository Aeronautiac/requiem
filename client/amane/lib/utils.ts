// Wall-clock time of an engine timestamp (unix ms). The one place the app decides what a
// moment looks like, so a message row and a log entry never disagree about it.
export function formatTime(ms: number): string {
	return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

// Human-readable duration from a millisecond count (engine Time is unix ms, so durations
// like a notebook-write delay come through in ms). Shows the largest one or two non-zero
// leading units: "2m 30s", "1h 1m", "800ms".
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
