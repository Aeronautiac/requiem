// One person row in the Players panel. The name is the trigger: clicking it opens the shared
// profile menu (contact / press conference / admin), so this row is just the name, its public
// statuses, and the channel read/send hint.
import { permsLabel, statusLabels } from "amane-client/text.ts";
import type { ChannelPerms } from "amane-client/game/types.ts";
import { statusAccent, tint } from "../../style.ts";
import { Entity } from "../Name.tsx";
import { useView } from "../game_ui.ts";

export function Player({ id, perms = null }: { id: string; perms?: ChannelPerms | null }) {
  const view = useView();
  const statuses = statusLabels(view.actor_statuses.get(id) ?? 0);
  const permsText = perms ? permsLabel(perms) : "";
  return (
    <div className="flex flex-wrap items-start justify-between gap-2 px-2 py-1.5 text-sm">
      <span className="flex min-w-0 flex-wrap items-center gap-1.5">
        <Entity of={{ kind: "player", id }} view={view} />
        {statuses.map((s) => (
          <span key={s} className="px-1 py-px text-xs uppercase tracking-wide" style={tint(statusAccent(s))}>
            {s}
          </span>
        ))}
      </span>
      {permsText && <span className="shrink-0 text-xs text-ink-dim">{permsText}</span>}
    </div>
  );
}
