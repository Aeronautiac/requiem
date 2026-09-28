// The main column: header, optional top-panel strip, the virtualized event list, and the composer.
// News is a selection with no backing channel of its own — world events render regardless of
// whether the News channel is held, since they live on the view rather than on the channel.
import type { ProfileKey } from "amane-client/bindings.ts";
import { slotKeyFromString, slotKeyToString } from "amane-client/bindings.ts";
import type { GameEvent } from "amane-client/game/types.ts";
import { PERM_SEND, ownPerms } from "amane-client/game/perms.ts";
import { feedEvents } from "amane-client/queries/feed.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { channelLabel, displayKey, mentionsViewer, refFromDisplay, refLabel } from "amane-client/text.ts";
import { useEffect, useRef, useState } from "react";
import type { VirtuosoHandle } from "react-virtuoso";
import { Virtuoso } from "react-virtuoso";
import { useNow, useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { Select } from "../../kit/Input.tsx";
import { tint } from "../../style.ts";
import { useGameUi, useView } from "../game_ui.ts";
import { MentionInput } from "../mentions/MentionInput.tsx";
import { EventAnnouncement } from "./EventAnnouncement.tsx";
import { Message } from "./Message.tsx";
import { NotebookPass } from "./NotebookPass.tsx";
import { NotebookWrite } from "./NotebookWrite.tsx";
// Another agent's widget strip: polls / prosecutions. No props.
import { TopPanel } from "../side/TopPanel.tsx";

// Discord-style chunking: only the sender header is dropped, and any non-message event in between
// breaks the chain — the run must be uninterrupted.
const GROUP_WINDOW_MS = 45_000;
function isGroupedMessage(prev: GameEvent | undefined, curr: GameEvent): boolean {
  if (!prev || !("Message" in prev.data) || !("Message" in curr.data)) return false;
  if (displayKey(prev.data.Message.sender_display) !== displayKey(curr.data.Message.sender_display)) return false;
  return curr.timestamp - prev.timestamp <= GROUP_WINDOW_MS;
}

export function ChannelView() {
  const session = useSession();
  const ui = useGameUi();
  const view = useView();
  // News may hold death beats stamped in the future; this keeps the feed moving as the clock
  // reaches them even with nothing new delivered.
  useNow(250);

  const [message_content, setMessageContent] = useState("");
  const [selected_profile_key, setSelectedProfileKey] = useState<string | null>(null);
  const [write_open, setWriteOpen] = useState(false);
  const [pass_open, setPassOpen] = useState(false);
  const virtuoso = useRef<VirtuosoHandle>(null);

  // System holds no channel perms of its own — it reads every viewport without being a member of
  // anything — so it is the one thing a view cannot answer for itself, and sending is the one
  // affordance that still asks who is looking.
  const is_admin = ui.viewer === "System";
  const is_news = ui.selected?.kind === "news";
  const backing_channel_id = is_news ? view.news_channel : ui.selected?.kind === "channel" ? ui.selected.id : null;
  const current_channel = backing_channel_id ? view.channels.get(backing_channel_id) : undefined;
  // What this view may do here at all, folded over every name it holds. The composer asks the
  // chosen name instead; this is only "is there a send box".
  const current_perms = backing_channel_id ? ownPerms(view.channel_views.get(backing_channel_id)?.own ?? []) : undefined;

  const is_bug = current_channel?.kind === "Bug";
  const is_contact_log = current_channel?.kind === "ContactLog";
  const is_log = current_channel?.kind === "Log";
  const is_log_autopsy = is_log && backing_channel_id?.startsWith("autopsy:") === true;
  // A feed rather than a room: no perms, no send box, no loggability control.
  const read_only_feed = current_channel != null && current_channel.kind !== "Standard";
  const archived = current_channel?.archived ?? false;
  const can_send = current_channel != null && !archived && !read_only_feed && (is_admin || (current_perms?.send ?? false));
  // Holding the channel is what grants the history. A view that has lost read still holds what it
  // was given, and `frozen` below is what says so.
  const can_read = current_channel != null;
  // This viewer left the channel's viewport, so what is shown is the last thing they heard.
  const frozen = backing_channel_id != null && view.frozen(view.channels.get(backing_channel_id)?.viewport ?? null);
  // News is not a channel, so it goes stale on its own terms.
  const news_frozen = is_news && view.frozen(view.world_events_viewport);

  const link = backing_channel_id ? view.channels.get(backing_channel_id)?.link : null;
  const notebook_id = link && "notebook" in link ? link.notebook : undefined;
  const loggable = backing_channel_id ? (view.channels.get(backing_channel_id)?.loggable ?? false) : false;
  const show_loggability = current_channel != null && !read_only_feed;
  const can_control_loggability = show_loggability && (is_admin || (current_perms?.loggability_control ?? false));
  const notebook_borrowed = notebook_id ? (view.notebooks.get(notebook_id)?.borrowed ?? false) : false;
  const notebook_fake = notebook_id ? view.notebooks.get(notebook_id)?.fake : undefined;
  // The notebook's only physical home is this very channel, so archiving it IS destroying it.
  const notebook_destroyed = notebook_id !== undefined && archived;

  // The names this view may speak as here. Send belongs to the name rather than to the person, so
  // holding a name that cannot talk is not an option to offer. Empty for System, which holds no
  // name anywhere and speaks as nobody.
  const sendable_profiles = backing_channel_id
    ? (view.channel_views.get(backing_channel_id)?.own ?? []).filter((profile) => (profile.perms & PERM_SEND) !== 0)
    : [];
  const sendable_keys = sendable_profiles.map((p) => slotKeyToString(p.profile_id));
  // Falls back to the first option whenever the stored choice isn't (or is no longer) valid — no
  // effect needed, since this is just what "the current selection" derives to on every render.
  const effective_profile_key = selected_profile_key && sendable_keys.includes(selected_profile_key) ? selected_profile_key : (sendable_keys[0] ?? null);

  const channel_name = backing_channel_id ? (() => {
    const name = view.channels.get(backing_channel_id)?.name;
    return name != null ? channelLabel(name) : null;
  })() : null;
  const header_name = channel_name ?? (is_news ? "News" : "");

  const now = view.game_time_now();
  const events = ui.selected ? feedEvents(view, ui.selected, now) : [];

  function senderProfile(): ProfileKey | null {
    if (ui.viewer === "System") return null;
    return sendable_profiles.find((p) => slotKeyToString(p.profile_id) === effective_profile_key)?.profile_id ?? null;
  }

  async function sendMessage() {
    if (!backing_channel_id || !message_content.trim()) return;
    await session.submit_action({
      actor: viewActor(ui.viewer),
      timestamp: Date.now(),
      payload: {
        SendMessage: { channel_id: slotKeyFromString(backing_channel_id), profile_id: senderProfile(), content: message_content.trim() },
      },
    });
    setMessageContent("");
  }

  async function toggleNotebookFake() {
    if (!notebook_id) return;
    await session.submit_action({
      actor: viewActor(ui.viewer),
      timestamp: Date.now(),
      payload: { SetNotebookFake: { notebook_id: slotKeyFromString(notebook_id), fake: !notebook_fake } },
    });
  }

  async function toggleLoggable() {
    if (!backing_channel_id) return;
    await session.submit_action({
      actor: viewActor(ui.viewer),
      timestamp: Date.now(),
      payload: { SetLoggable: { channel_id: slotKeyFromString(backing_channel_id), loggable: !loggable } },
    });
  }

  // A poll-panel jump lands here with the poll to reveal.
  useEffect(() => {
    if (!ui.jump_poll) return;
    const index = events.findIndex((e) => "PollNotice" in e.data && e.data.PollNotice.poll_id === ui.jump_poll);
    if (index >= 0) virtuoso.current?.scrollToIndex({ index, align: "center", behavior: "smooth" });
    ui.setJumpPoll(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ui.jump_poll]);

  return (
    <div className="flex h-full min-h-0 w-full flex-1 flex-col bg-surface text-ink">
      <header className="flex h-10 shrink-0 items-center gap-2 border-b border-edge px-3">
        <button
          type="button"
          className="flex h-8 w-8 shrink-0 items-center justify-center lg:hidden"
          title="Channels"
          onClick={() => ui.setDrawer("left")}
        >
          ☰
        </button>
        <span className="min-w-0 flex-1 truncate text-base font-semibold text-ink">{header_name}</span>

        {archived && <span className="shrink-0 border border-edge bg-panel px-1.5 py-0.5 text-xs text-ink-dim">archived</span>}

        <div className="ml-auto flex shrink-0 items-center gap-2">
          {notebook_destroyed && (
            <span className="border border-edge bg-panel px-2 py-0.5 text-xs font-medium text-ink-dim" title="This notebook no longer exists.">
              Destroyed
            </span>
          )}
          {notebook_borrowed && !notebook_destroyed && (
            <span className="px-2 py-0.5 text-xs font-medium" style={tint("var(--color-status-ipp)")} title="This notebook is currently on loan (being borrowed).">
              Borrowed
            </span>
          )}
          {notebook_fake !== undefined && !notebook_destroyed && (
            <NotebookFakeBadge fake={notebook_fake} editable={is_admin} onToggle={toggleNotebookFake} />
          )}
          {show_loggability && <LoggableBadge loggable={loggable} editable={can_control_loggability} onToggle={toggleLoggable} />}
        </div>

        <button
          type="button"
          className="flex h-8 w-8 shrink-0 items-center justify-center lg:hidden"
          title="People"
          onClick={() => ui.setDrawer("right")}
        >
          ⋯
        </button>
      </header>

      {ui.top_panel && <TopPanel />}

      {!ui.selected ? (
        <div className="flex flex-1 items-center justify-center text-ink-dim">Select a channel</div>
      ) : (
        <>
          <Virtuoso
            ref={virtuoso}
            className="min-h-0 flex-1"
            data={events}
            followOutput="auto"
            computeItemKey={(index, event) => `${index}-${event.timestamp}`}
            itemContent={(index, event) => {
              if ("Message" in event.data) {
                const msg = event.data.Message;
                return (
                  <Message
                    senderDisplay={msg.sender_display}
                    content={msg.content}
                    view={view}
                    timestamp={event.timestamp}
                    grouped={isGroupedMessage(events[index - 1], event)}
                    last={!events[index + 1] || !isGroupedMessage(event, events[index + 1])}
                    mentioned={mentionsViewer(view, msg.content)}
                  />
                );
              }
              return <EventAnnouncement event={event} view={view} timestamp={event.timestamp} />;
            }}
            components={{
              Footer: () => (
                <div className="pb-2">
                  {is_news ? (
                    <>
                      {news_frozen && (
                        <p className="px-4 py-3 text-center text-sm" style={{ color: "var(--color-status-bugged)" }}>
                          You are no longer receiving news. Everything above is what you last heard.
                        </p>
                      )}
                      {!can_read && (
                        <p className="px-4 py-3 text-center text-sm text-ink-dim">
                          You don't have access to this channel. Announcements above are game events and are always shown here — but you can't see
                          chat messages.
                        </p>
                      )}
                    </>
                  ) : (
                    !can_read && (
                      <p className="px-4 py-3 text-center text-sm text-ink-dim">
                        You no longer have read access to this channel. Everything above is what you were given.
                      </p>
                    )
                  )}
                  {archived && <p className="px-4 py-3 text-center text-sm text-ink-dim">This channel has been archived.</p>}
                </div>
              ),
            }}
          />

          <footer className="shrink-0 px-3 pb-2 pt-1">
            <div className="flex items-end gap-2">
              {can_send && sendable_profiles.length > 1 && (
                <Select
                  value={effective_profile_key ?? ""}
                  onChange={(e) => setSelectedProfileKey(e.target.value)}
                  className="w-auto shrink-0"
                  options={sendable_profiles.map((p) => ({
                    value: slotKeyToString(p.profile_id),
                    label: refLabel(refFromDisplay(p.display, view.orgs), view.players),
                  }))}
                />
              )}

              <div className="min-w-0 flex-1">
                {can_send ? (
                  <div className="flex items-end gap-2">
                    <MentionInput value={message_content} onChange={setMessageContent} onSubmit={sendMessage} placeholder={`Message ${channel_name ?? ""}`} />
                    <Button size="sm" onClick={sendMessage}>
                      Send
                    </Button>
                  </div>
                ) : (
                  <ComposerNotice
                    text={
                      // Frozen outranks the read-only blurbs: "this is no longer live" is the more
                      // important of the two facts.
                      frozen && read_only_feed
                        ? "This feed no longer reaches you. Everything above is what it last relayed."
                        : is_bug
                          ? archived
                            ? "This surveillance feed is no longer active."
                            : "Read-only surveillance feed."
                          : is_contact_log
                            ? "Read-only contact log. Names here are how each contact appeared, not who was really behind it."
                            : is_log
                              ? `${is_log_autopsy ? "Autopsy — this actor's written record." : "Tap-in — what was written down on the tapped line."} Read-only.`
                              : archived
                                ? "This channel is archived and read-only."
                                : frozen
                                  ? "You are no longer in this channel. Everything above is what you last saw."
                                  : "You do not have permission to send messages in this channel."
                    }
                  />
                )}
              </div>

              {notebook_id && !notebook_destroyed && (
                <>
                  <Button size="sm" variant="ghost" onClick={() => setPassOpen(true)}>
                    Pass
                  </Button>
                  <Button size="sm" variant="danger" onClick={() => setWriteOpen(true)}>
                    Write
                  </Button>
                </>
              )}
            </div>
          </footer>

          {notebook_id && !notebook_destroyed && (
            <>
              <NotebookWrite open={write_open} onClose={() => setWriteOpen(false)} notebookId={slotKeyFromString(notebook_id)} />
              <NotebookPass open={pass_open} onClose={() => setPassOpen(false)} notebookId={slotKeyFromString(notebook_id)} />
            </>
          )}
        </>
      )}
    </div>
  );
}

function ComposerNotice({ text }: { text: string }) {
  return <p className="border border-edge bg-panel px-4 py-2.5 text-center text-sm italic text-ink-dim">{text}</p>;
}

function NotebookFakeBadge({ fake, editable, onToggle }: { fake: boolean; editable: boolean; onToggle: () => void }) {
  const color = fake ? "var(--color-status-kidnapped)" : "var(--color-event-revival)";
  const label = fake ? "Fake" : "Real";
  if (editable) {
    return (
      <button type="button" className="px-2 py-0.5 text-xs font-medium hover:brightness-125" style={tint(color)} title="Toggle whether this notebook is a decoy — a fake book's writes cannot kill." onClick={onToggle}>
        {label}
      </button>
    );
  }
  return (
    <span className="px-2 py-0.5 text-xs font-medium" style={tint(color)} title="A fake notebook's writes cannot kill. Only you and the host know this book's nature.">
      {label}
    </span>
  );
}

function LoggableBadge({ loggable, editable, onToggle }: { loggable: boolean; editable: boolean; onToggle: () => void }) {
  const style = loggable ? tint("var(--color-event-surveillance)") : undefined;
  const className = `px-2 py-0.5 text-xs font-medium ${loggable ? "" : "bg-panel text-ink-dim"}`;
  const label = `Logging ${loggable ? "on" : "off"}`;
  if (editable) {
    return (
      <button type="button" className={`${className} hover:brightness-125`} style={style} title="Toggle whether messages sent here can be logged (autopsied / relayed to bugs)" onClick={onToggle}>
        {label}
      </button>
    );
  }
  return (
    <span className={className} style={style} title="Whether messages sent here can be logged (autopsied / relayed to bugs)">
      {label}
    </span>
  );
}
