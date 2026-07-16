import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
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

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
