// The left rail: every channel category this view holds, in fixed sidebar order, plus the two
// affordances that create something (a group chat, a personal channel) when the view is entitled to.
import type { ActionRequest } from "amane-client/bindings.ts";
import type { ChannelCategory } from "amane-client/game/types.ts";
import { CHANNEL_CATEGORIES } from "amane-client/game/types.ts";
import { channelLabel } from "amane-client/text.ts";
import { channelCategories, createGroupchatAbility } from "amane-client/queries/channels.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Section } from "../../kit/Section.tsx";
import { useSession } from "../../hooks.ts";
import { useGameUi, useView } from "../game_ui.ts";

const CATEGORY_LABELS: Record<ChannelCategory, string> = {
  Raw: "Misc",
  Lounge: "Lounges",
  Groupchat: "Group Chats",
  Notebook: "Notebooks",
  Role: "Roles",
  World: "World",
  Org: "Organizations",
  Prosecution: "Trials",
  Kidnapping: "Kidnapping",
  Logs: "Logs",
  Personal: "Personal",
};

export function ChannelList() {
  const ui = useGameUi();
  const view = useView();
  const is_admin = ui.viewer === "System";
  const categories = channelCategories(view);
  const gc_ability_id = createGroupchatAbility(view);

  return (
    <div className="flex flex-col pb-2">
      {CHANNEL_CATEGORIES.map((category) => {
        const keys = categories.get(category) ?? [];
        // Categories that show even when empty: World holds News, Lounges is there purely so the
        // sidebar reads consistently above Group Chats, and Personal/Groupchat keep their create
        // buttons reachable.
        const show =
          keys.length > 0 ||
          category === "World" ||
          category === "Lounge" ||
          (category === "Personal" && !is_admin) ||
          (category === "Groupchat" && gc_ability_id !== undefined);
        if (!show) return null;

        return (
          <Section key={category} label={CATEGORY_LABELS[category]}>
            <div className="flex flex-col gap-0.5 px-1 py-1">
              {category === "World" && (
                <ChannelRow label="News" active={ui.selected?.kind === "news"} onClick={() => ui.select({ kind: "news" })} />
              )}

              {keys.map((key) => {
                const channel = view.channels.get(key)!;
                return (
                  <ChannelRow
                    key={key}
                    label={channelLabel(channel.name)}
                    archived={channel.archived}
                    frozen={channel.viewport !== null && view.frozen(channel.viewport)}
                    active={ui.selected !== null && ui.selected.kind === "channel" && ui.selected.id === key}
                    onClick={() => ui.select({ kind: "channel", id: key })}
                  />
                );
              })}

              {category === "Groupchat" && gc_ability_id !== undefined && <CreateGroupchatRow abilityId={gc_ability_id} />}
              {category === "Personal" && !is_admin && <CreatePersonalChannelRow />}
            </div>
          </Section>
        );
      })}
    </div>
  );
}

function ChannelRow({
  label,
  active,
  archived = false,
  frozen = false,
  onClick,
}: {
  label: string;
  active: boolean;
  archived?: boolean;
  frozen?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      title={frozen ? "No longer reaching you — showing what you last saw." : undefined}
      onClick={onClick}
      className={`min-h-8 w-full truncate px-2 py-1.5 text-left text-sm leading-none ${
        active ? "bg-raised text-ink" : archived ? "text-ink-dim hover:bg-raised" : "text-ink hover:bg-raised"
      }`}
    >
      {label}
      {frozen && <span className="ml-1 text-ink-dim">·stale</span>}
    </button>
  );
}

// Driven by the viewer's own ability, exactly like Contact. A view holding no such ability shows no
// button, which covers System without asking whether it is System.
function CreateGroupchatRow({ abilityId }: { abilityId: string }) {
  const session = useSession();
  const ui = useGameUi();
  const flash = useFlash();

  async function create() {
    const request: ActionRequest = {
      actor: viewActor(ui.viewer),
      timestamp: Date.now(),
      payload: { UseAbility: { ability_id: slotKeyFromString(abilityId), ability_args: { CreateGroupchat: {} } } },
    };
    const reply = await session.submit_action(request);
    flash.reply(reply, "Group chat created.");
  }

  return (
    <div className="flex flex-col gap-1">
      <button type="button" onClick={create} className="min-h-8 w-full truncate px-2 py-1.5 text-left text-sm leading-none text-ink-dim hover:bg-raised hover:text-ink">
        + Create group chat
      </button>
      <div className="px-2">
        <FlashLine flash={flash} />
      </div>
    </div>
  );
}

// A direct player action rather than an ability, so admins get no button. The engine caps how many a
// player may hold and rejects past the limit; we just surface the error.
function CreatePersonalChannelRow() {
  const session = useSession();
  const ui = useGameUi();
  const flash = useFlash();

  async function create() {
    const request: ActionRequest = {
      actor: viewActor(ui.viewer),
      timestamp: Date.now(),
      payload: { CreatePersonalChannel: {} },
    };
    const reply = await session.submit_action(request);
    flash.reply(reply, "Personal channel created.");
  }

  return (
    <div className="flex flex-col gap-1">
      <button type="button" onClick={create} className="min-h-8 w-full truncate px-2 py-1.5 text-left text-sm leading-none text-ink-dim hover:bg-raised hover:text-ink">
        + Add personal channel
      </button>
      <div className="px-2">
        <FlashLine flash={flash} />
      </div>
    </div>
  );
}
