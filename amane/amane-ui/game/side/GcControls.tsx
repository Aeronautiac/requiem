// The group chat owner's controls for the selected channel: add a member, remove one, or hand off
// ownership. Renders nothing unless the selected channel is a group chat this view owns.
import type { Action } from "amane-client/bindings.ts";
import { slotKeyFromString, slotKeyToString } from "amane-client/bindings.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { refFromDisplay, refLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Section } from "../../kit/Section.tsx";
import { useGameUi, useView } from "../game_ui.ts";
import { Entity } from "../Name.tsx";

export function GcControls() {
  const ui = useGameUi();
  const view = useView();
  const session = useSession();
  const flash = useFlash();

  const channelId = ui.selected?.kind === "channel" ? ui.selected.id : null;
  const link = channelId ? view.channels.get(channelId)?.link : null;
  const gcKey = link && "gc" in link ? link.gc : null;
  const isOwner = gcKey !== null && view.owned_gcs.has(gcKey);
  if (!gcKey || !isOwner) return null;

  const members: { id: string; name: string }[] = [];
  for (const profile of view.channel_views.get(channelId!)?.roster ?? []) {
    const d = profile.display;
    if (typeof d !== "string" && "Raw" in d) {
      const id = slotKeyToString(d.Raw);
      if (id === ui.viewer) continue;
      members.push({ id, name: refLabel(refFromDisplay(d, view.orgs), view.players) });
    }
  }
  const memberIds = new Set(members.map((m) => m.id));
  const candidates = [...view.players.keys()].filter((id) => id !== ui.viewer && !memberIds.has(id));

  async function send(payload: Action, ok: string) {
    const reply = await session.submit_action({ actor: viewActor(ui.viewer), timestamp: Date.now(), payload });
    flash.reply(reply, ok);
  }
  function add(playerId: string) {
    send(
      { AddToGroupchat: { groupchat_id: slotKeyFromString(gcKey!), player_id: slotKeyFromString(playerId), owner: false } },
      "Added.",
    );
  }
  function remove(playerId: string) {
    send(
      { RemoveFromGroupchat: { groupchat_id: slotKeyFromString(gcKey!), player_id: slotKeyFromString(playerId) } },
      "Removed.",
    );
  }
  function transfer(playerId: string) {
    send(
      { SetGroupchatOwner: { groupchat_id: slotKeyFromString(gcKey!), owner: slotKeyFromString(playerId) } },
      "Ownership transferred.",
    );
  }

  return (
    <Section label="Group Chat Controls">
      <p className="px-3 pt-2 pb-0.5 text-xs font-medium uppercase tracking-wide text-ink-dim">Add member</p>
      {candidates.length === 0 ? (
        <p className="px-3 py-1 text-sm text-ink-dim">No one to add</p>
      ) : (
        candidates.map((id) => (
          <button
            key={id}
            type="button"
            className="flex min-h-8 w-full items-center justify-between px-3 py-1.5 text-sm text-ink hover:bg-raised"
            onClick={() => add(id)}
          >
            <span className="min-w-0 truncate">
              <Entity of={{ kind: "player", id }} view={view} menu={false} />
            </span>
            <span className="shrink-0 text-sm text-ink-dim">add</span>
          </button>
        ))
      )}

      <p className="px-3 pt-2 pb-0.5 text-xs font-medium uppercase tracking-wide text-ink-dim">Members</p>
      {members.length === 0 ? (
        <p className="px-3 py-1 text-sm text-ink-dim">No other members</p>
      ) : (
        members.map((m) => (
          <div key={m.id} className="flex flex-wrap items-center justify-between gap-2 px-3 py-1.5 text-sm text-ink">
            <span className="min-w-0 truncate">{m.name}</span>
            <span className="flex shrink-0 gap-1">
              <Button variant="ghost" size="sm" title="Make owner" onClick={() => transfer(m.id)}>
                owner
              </Button>
              <Button variant="danger" size="sm" title="Remove from group chat" onClick={() => remove(m.id)}>
                remove
              </Button>
            </span>
          </div>
        ))
      )}

      <div className="px-3 py-1.5">
        <FlashLine flash={flash} />
      </div>
    </Section>
  );
}
