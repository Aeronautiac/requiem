// The right rail's roster: who is in the open channel (or News) now, split by whether they can do
// anything there, and everyone else this view knows about.
import { slotKeyToString } from "amane-client/bindings.ts";
import { ownPerms } from "amane-client/game/perms.ts";
import type { MemberRow } from "amane-client/queries/players.ts";
import { channelRoster, otherPlayers, trueOwners } from "amane-client/queries/players.ts";
import { permsLabel, refFromDisplay } from "amane-client/text.ts";
import { Section } from "../../kit/Section.tsx";
import { useGameUi, useView } from "../game_ui.ts";
import { Entity } from "../Name.tsx";
import { Player } from "./Player.tsx";

export function Players() {
  const ui = useGameUi();
  const view = useView();
  const channelId =
    ui.selected?.kind === "news" ? view.news_channel : ui.selected?.kind === "channel" ? ui.selected.id : null;
  const { active, silent } = channelRoster(view, channelId);
  const others = otherPlayers(view, channelId);

  function memberRow(m: MemberRow) {
    const perms = ownPerms([m.profile]);
    const owners = trueOwners(view, channelId, m.profile);
    return (
      <div key={slotKeyToString(m.profile.profile_id)} className="flex flex-col">
        {m.playerId ? (
          <Player id={m.playerId} perms={perms} />
        ) : (
          // Nothing to contact or inspect. A role/org/anonymous display colours by its own entity
          // accent, exactly as that same display reads in a chat row.
          <div className="flex items-start justify-between gap-2 px-2 py-1.5 text-sm">
            <span className="min-w-0 break-words">
              <Entity of={refFromDisplay(m.profile.display, view.orgs)} view={view} menu={false} />
            </span>
            {permsLabel(perms) && <span className="shrink-0 text-xs text-ink-dim">{permsLabel(perms)}</span>}
          </div>
        )}
        {owners && owners.length > 0 && (
          // Admin only: the true holders of the name above, greyed and stacked so they read as an
          // annotation on that name rather than as player entries of their own.
          <div className="flex flex-col gap-0.5 pb-0.5 pl-3">
            {owners.map((name, i) => (
              <span key={i} className="text-xs text-ink-dim">
                {name}
              </span>
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <>
      <Section label="Channel Members">
        {!channelId ? (
          <p className="px-2 py-1 text-sm text-ink-dim">No channel selected</p>
        ) : active.length === 0 && silent.length === 0 ? (
          <p className="px-2 py-1 text-sm text-ink-dim">No members</p>
        ) : (
          <>
            {active.map(memberRow)}
            {silent.length > 0 && (
              <>
                <p className="px-2 pt-2 pb-0.5 text-xs font-medium uppercase tracking-wide text-ink-dim">
                  No permissions
                </p>
                {silent.map(memberRow)}
              </>
            )}
          </>
        )}
      </Section>
      <Section label="Other Players">
        {others.length === 0 ? (
          <p className="px-2 py-1 text-sm text-ink-dim">No other players</p>
        ) : (
          others.map((id) => <Player key={id} id={id} />)
        )}
      </Section>
    </>
  );
}
