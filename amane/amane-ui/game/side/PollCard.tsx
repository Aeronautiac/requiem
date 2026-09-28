// One poll, rendered the same way whether it sits inline in its home channel or in the top panel.
// `variant` only changes width/jump target: inline knows it is already in the channel, so its jump
// opens the panel; the panel jumps to the home channel.
import type { Action, PollOptionIndex, PollOptionLabel, PollParent } from "amane-client/bindings.ts";
import { slotKeyFromString, slotKeyToString } from "amane-client/bindings.ts";
import type { PollData, PollView } from "amane-client/game/types.ts";
import { pollHomeChannel } from "amane-client/queries/polls.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { channelLabel, orgDisplayName, pollSubjectArgs, pollSubjectHeading, refFromKey, refLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi, useView } from "../game_ui.ts";

export function PollCard({
  id,
  data,
  pollView,
  frozen,
  variant,
}: {
  id: string;
  data: PollData;
  pollView: PollView | null;
  frozen: boolean;
  variant: "inline" | "panel";
}) {
  const ui = useGameUi();
  const view = useView();
  const session = useSession();
  const flash = useFlash();

  function parentLabel(parent: PollParent): string {
    if (parent === "World") return "Everyone";
    if ("Org" in parent) {
      const org = view.orgs.get(slotKeyToString(parent.Org));
      return org ? orgDisplayName(org.name) : "Org";
    }
    const ch = view.channels.get(slotKeyToString(parent.Channel));
    return ch ? channelLabel(ch.name) : "Channel";
  }
  function optionLabel(label: PollOptionLabel): string {
    return typeof label === "string" ? label : label.Generic;
  }

  const args = pollSubjectArgs(data.subject, view.players);

  function jump() {
    if (variant === "inline") {
      // Already in the channel — jump means "see it in the full panel with every other poll".
      if (ui.top_panel !== "polls") ui.togglePanel("polls");
      return;
    }
    // Land on the poll's home channel, drop the panel, and ask that channel to scroll the poll's
    // inline card into view.
    const home = pollHomeChannel(view, data.parent);
    if (home === "news") ui.select({ kind: "news" });
    else if (home) ui.select({ kind: "channel", id: home });
    if (ui.top_panel === "polls") ui.togglePanel("polls");
    ui.setJumpPoll(id);
  }

  async function send(payload: Action, ok: string) {
    const reply = await session.submit_action({ actor: viewActor(ui.viewer), timestamp: Date.now(), payload });
    flash.reply(reply, ok);
  }
  function vote(option: PollOptionIndex, label: string) {
    send({ AddVote: { poll_id: slotKeyFromString(id), option } }, `Voted ${label}.`);
  }
  function retract() {
    send({ RemoveVote: { poll_id: slotKeyFromString(id) } }, "Vote retracted.");
  }

  return (
    <div className={`flex flex-col gap-2 border border-edge bg-panel p-3 ${variant === "panel" ? "w-[min(18rem,85vw)] shrink-0" : "w-full"}`}>
      <div className="flex items-start justify-between gap-2">
        <span className="text-sm font-medium text-ink">{pollSubjectHeading(data.subject, view.players)}</span>
        <button type="button" className="shrink-0 text-xs uppercase tracking-wide text-ink-dim hover:text-ink" onClick={jump}>
          {variant === "inline" ? "open ▸" : "jump ▸"}
        </button>
      </div>

      {args.length > 0 && (
        <div className="flex flex-col gap-0.5">
          {args.map((arg) => (
            <span key={arg.label} className="text-xs text-ink-dim">
              <span className="text-ink-dim">{arg.label}:</span> {arg.value}
            </span>
          ))}
        </div>
      )}

      <div className="flex flex-wrap items-center gap-x-2 text-xs text-ink-dim">
        <span className="bg-surface px-1.5 py-0.5 text-ink-dim">{parentLabel(data.parent)}</span>
        {data.opener && <span>started by {refLabel(refFromKey(data.opener, view.players, view.orgs), view.players)}</span>}
      </div>

      <div className="flex flex-col gap-0.5 text-xs text-ink-dim">
        {data.options.map((option, i) => (
          <span key={i} className={pollView?.own_vote === i ? "text-ink" : undefined}>
            {optionLabel(option.label)} {option.weight}
          </span>
        ))}
        <span>· of {data.potential}</span>
      </div>

      {frozen ? (
        <span className="text-sm italic" style={{ color: "var(--color-event-alarm)" }}>
          no longer visible to you — last known tally
        </span>
      ) : pollView === null ? (
        <span className="text-sm italic text-ink-dim">observing</span>
      ) : !pollView.eligible ? (
        <span className="text-sm italic text-ink-dim">you can't vote in this poll</span>
      ) : pollView.own_vote === null ? (
        <div className="flex flex-wrap gap-1">
          {data.options.map((option, i) => (
            <Button key={i} variant="ghost" size="sm" className="flex-1" onClick={() => vote(i, optionLabel(option.label))}>
              {optionLabel(option.label)}
            </Button>
          ))}
        </div>
      ) : (
        <div className="flex items-center justify-between gap-2">
          <span className="text-sm text-ink">you voted {optionLabel(data.options[pollView.own_vote]?.label ?? { Generic: "?" })}</span>
          <Button variant="ghost" size="sm" onClick={retract}>
            retract
          </Button>
        </div>
      )}

      <FlashLine flash={flash} />
    </div>
  );
}
