import type { ActionActor, ActorKey } from "./bindings";
import { slotKeyFromString, slotKeyToString } from "./bindings";

export type Viewer = "Admin" | ActorKey;

export function viewerKey(viewer: Viewer): string {
    return viewer === "Admin" ? "Admin" : slotKeyToString(viewer);
}

export function viewerToActor(key: string): ActionActor {
    if (key === "Admin") return "Admin";
    return { Player: slotKeyFromString(key) };
}
