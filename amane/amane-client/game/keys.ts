// Stable string keys for the record kinds that share the channels map with real channels.
import type { BugKey, ContactLogType, LogType } from "../bindings.ts";
import { slotKeyToString } from "../bindings.ts";

// The stable per-view key for a log record, derived from what it is. An autopsy is of one actor's
// record; a tap-in is of one channel by contact id.
export function logDumpKey(log_type: LogType): string {
  if ("Autopsy" in log_type) return `autopsy:${slotKeyToString(log_type.Autopsy)}`;
  return `tapin:${log_type.TapIn}`;
}

// Bugs live in their own BugKey slot space, which can collide with real ChannelKeys, so the prefix
// keeps them separate in the one channels map.
export function bugChannelKey(bug_id: BugKey): string {
  return `bug:${slotKeyToString(bug_id)}`;
}

// There are exactly three contact-log records (Full/Even/Odd), keyed by which one they are rather
// than by any passive — the record is a world singleton, and the same feed reaches a linked reader
// who never sees which passive fed it.
export function contactLogChannelKey(kind: ContactLogType): string {
  return `contacts:${kind}`;
}
