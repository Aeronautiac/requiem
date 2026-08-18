<script lang="ts">
  import { getContext, tick } from "svelte";
  import MentionInput from "./MentionInput.svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import {
    PERM_SEND,
    actorDisplayLabel,
    channelLabel,
    displayKey,
    isReadOnlyKind,
    mentionsViewer,
    ownPerms,
    t,
  } from "../../game/helpers.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { GameEvent } from "../../game/types";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type {
    ActionRequest,
    PollOutcome,
    PollSubject,
    ProfileKey,
  } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import EventAnnouncement from "./announcements/EventAnnouncement.svelte";
  import Button from "../kit/Button.svelte";
  import Message from "./Message.svelte";
  import TopPanel from "./TopPanel.svelte";
  import NotebookWrite from "./NotebookWrite.svelte";
  import NotebookPass from "./NotebookPass.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  let message_content = $state("");
  let channel_name = $derived(get_channel_name());
  let scroller = $state<HTMLElement>();

  // System holds no channel perms of its own — it reads every viewport without being a member of
  // anything — so it is the one thing a view cannot answer for itself, and sending is the one
  // affordance that still asks who is looking.
  const is_admin = $derived(ui.viewer === "Admin");

  // News is its own selection, not a channel: it always exists on the frontend and its backing
  // channel may be absent. That lookup is never news's identity, and world events render either
  // way since they live per-view rather than on the channel.
  const is_news = $derived(ui.is_news);
  const backing_channel_id = $derived(
    is_news ? view.news_channel_id : ui.selected_channel,
  );

  const current_channel = $derived(
    backing_channel_id ? view.channel(backing_channel_id) : undefined,
  );
  // What this view may do here at all, folded over every name it holds. The composer asks the
  // chosen name instead; this is only "is there a send box".
  const current_perms = $derived(
    backing_channel_id
      ? ownPerms(view.channel_views.get(backing_channel_id)?.own ?? [])
      : undefined,
  );

  const is_bug = $derived(current_channel?.kind === "Bug");
  const is_contact_log = $derived(current_channel?.kind === "ContactLog");
  const is_log = $derived(current_channel?.kind === "Log");
  // Which kind of record is open — an autopsy of one actor's record, or a tap-in off a line.
  const is_log_autopsy = $derived(is_log && backing_channel_id?.startsWith("autopsy:") === true);
  // A feed rather than a room: no perms, no send box, no loggability control. Always readable to
  // whoever it was listed for, since visibility is gated in the sidebar instead.
  const read_only_feed = $derived(
    current_channel != null && isReadOnlyKind(current_channel.kind),
  );
  const archived = $derived(current_channel?.archived ?? false);
  // News with no backing channel lands here as false, so only its event log shows. Archived
  // channels are read-only for everyone, admin included.
  const can_send = $derived(
    current_channel != null &&
      !archived &&
      !read_only_feed &&
      (is_admin || (current_perms?.send ?? false)),
  );
  // Holding the channel is what grants the history. A view that has lost read still holds what it
  // was given, and `frozen` below is what says so.
  const can_read = $derived(current_channel != null);
  // This viewer left the channel's viewport, so what is shown is the last thing they heard.
  // Distinct from `archived`, which means the channel is over for everyone — this is personal, and
  // the channel may well still be busy without them.
  //
  // Asked of read-only feeds too: a bug you have lost or a contact log you no longer reach stops
  // relaying exactly the way a channel stops carrying messages, and a surveillance feed that has
  // quietly gone dead is the worst thing on the screen to mistake for a live one.
  const frozen = $derived(
    backing_channel_id != null &&
      view.frozen(view.viewport_of(backing_channel_id)),
  );
  // News is not a channel, so it goes stale on its own terms: world events ride the world-events
  // viewport, and a viewer who has left it keeps every event they were given while receiving no
  // more. Without this the feed just stops, which reads as "nothing has happened" — whether the
  // viewer lost presence or the world went dark.
  const news_frozen = $derived(
    is_news && view.frozen(view.world_events_viewport()),
  );
  // Notebook-ness isn't a channel kind. A non-undefined notebook_id both identifies the channel
  // as a notebook and gives the Write affordance its target.
  const notebook_id = $derived(
    backing_channel_id ? view.notebook_of(backing_channel_id) : undefined,
  );
  const loggable = $derived(
    backing_channel_id ? view.is_loggable(backing_channel_id) : false,
  );
  // Shown on any real engine channel; read-only feeds aren't engine channels, so the control
  // never applies there. It becomes an interactive toggle only with loggability control.
  const show_loggability = $derived(current_channel != null && !read_only_feed);
  const can_control_loggability = $derived(
    show_loggability &&
      (is_admin || (current_perms?.loggability_control ?? false)),
  );
  const notebook_borrowed = $derived(
    notebook_id ? view.is_notebook_borrowed(notebook_id) : false,
  );
  // undefined for a view that was never told (a borrower/inheritor) — no badge at all, not a claim
  // the book is genuine. The admin always receives it, so the badge doubles as the toggle.
  const notebook_fake = $derived(
    notebook_id ? view.notebook_fake(notebook_id) : undefined,
  );
  let write_open = $state(false);
  let pass_open = $state(false);

  // The names this view may speak as here. Send belongs to the name rather than to the person, so
  // holding a name that cannot talk is not an option to offer. Empty for System, which holds no
  // name anywhere and speaks as nobody.
  const sendable_profiles = $derived(
    backing_channel_id
      ? (view.channel_views.get(backing_channel_id)?.own ?? []).filter(
          (profile) => (profile.perms & PERM_SEND) !== 0,
        )
      : [],
  );
  let selected_profile_key = $state<string | null>(null);

  // Keep the selection valid as the channel, and thus the options, changes.
  $effect(() => {
    const keys = sendable_profiles.map((p) => slotKeyToString(p.profile_id));
    if (!selected_profile_key || !keys.includes(selected_profile_key)) {
      selected_profile_key = keys[0] ?? null;
    }
  });

  // News may be selected without a backing channel object.
  const header_name = $derived(channel_name ?? (is_news ? "News" : ""));

  function get_channel_name(): string | null {
    const name = backing_channel_id
      ? view.channel(backing_channel_id)?.name
      : null;
    return name != null ? channelLabel(name) : null;
  }

  // The name to speak as, or null for the host speaking as nobody — which shows as System and is
  // the one case that needs no name in the channel at all.
  function sender_profile(): ProfileKey | null {
    if (ui.viewer === "Admin") return null;
    return (
      sendable_profiles.find(
        (p) => slotKeyToString(p.profile_id) === selected_profile_key,
      )?.profile_id ?? null
    );
  }

  // Discord-style chunking: only the sender header is dropped, and any non-message event in
  // between breaks the chain — the run must be uninterrupted.
  const GROUP_WINDOW_MS = 45_000;
  function is_grouped_message(
    prev: GameEvent | undefined,
    curr: GameEvent,
  ): boolean {
    if (!prev || !("Message" in prev.data) || !("Message" in curr.data))
      return false;
    if (
      displayKey(prev.data.Message.sender_display) !==
      displayKey(curr.data.Message.sender_display)
    )
      return false;
    return curr.timestamp - prev.timestamp <= GROUP_WINDOW_MS;
  }

  async function send_message() {
    // can_send already gates the box on the channel existing, so this is only defensive.
    if (!backing_channel_id || !message_content.trim()) return;
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        SendMessage: {
          channel_id: slotKeyFromString(backing_channel_id),
          profile_id: sender_profile(),
          content: message_content.trim(),
        },
      },
    };
    await session.submit_action(request);
    message_content = "";
    console.log("message sent");
  }

  async function toggle_notebook_fake() {
    if (!notebook_id) return;
    await session.submit_action({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        SetNotebookFake: {
          notebook_id: slotKeyFromString(notebook_id),
          fake: !notebook_fake,
        },
      },
    });
  }

  async function toggle_loggable() {
    if (!backing_channel_id) return;
    await session.submit_action({
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        SetLoggable: {
          channel_id: slotKeyFromString(backing_channel_id),
          loggable: !loggable,
        },
      },
    });
  }

  const events: GameEvent[] = $derived.by(() => {
    // News has no channel key but still counts as selected, so gate on the selection itself.
    if (!ui.selected) {
      return [];
    }

    let accum: GameEvent[] = [];

    // World events are not channel messages, so news renders them whether or not the news
    // channel exists or the viewer has any perms for it.
    if (is_news) {
      accum = accum.concat(view.events);
    }

    // Everything the channel holds, with no filter. This view was delivered exactly what it was
    // entitled to and nothing after that, so there is no later cutoff to apply.
    if (current_channel) accum = accum.concat(current_channel.events);

    accum.sort((a, b) => a.timestamp - b.timestamp);

    return accum;
  });

  // The tail window: a channel with a long history renders only the most recent `shown` events,
  // so loading a busy channel doesn't mount its whole history as DOM at once. Scrolling up (or
  // clicking "Load earlier") grows the window by WINDOW at a time. `start` is the first index of
  // the window, and `visible` is that slice of the full, still-unsliced `events` array — grouping
  // and the `last` tail are computed against the full array so a window boundary never breaks a
  // run of consecutive messages.
  const WINDOW = 200;
  let shown = $state(WINDOW);
  const start = $derived(Math.max(0, events.length - shown));
  const visible = $derived(events.slice(start));

  // Grow the window by one chunk, keeping whatever was on screen in place: the prepended content
  // pushes the old top down, so we compensate by scrolling down by exactly the height added.
  async function load_earlier() {
    if (start === 0 || !scroller) return;
    const prev_height = scroller.scrollHeight;
    shown = Math.min(events.length, shown + WINDOW);
    await tick();
    scroller.scrollTop += scroller.scrollHeight - prev_height;
  }

  // The <main> element is reused across channels, so its scrollTop would otherwise carry over.
  $effect(() => {
    ui.selected; // track
    scroller?.scrollTo({ top: scroller.scrollHeight });
  });

  // Stick to the bottom on new messages, but only if the user is already near it, so scrolling up
  // to read history isn't yanked back down. Runs after the DOM flush, so scrollHeight already
  // includes the new content.
  $effect(() => {
    events.length; // track
    if (!scroller) return;
    const near_bottom =
      scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 100;
    if (near_bottom) scroller.scrollTo({ top: scroller.scrollHeight });
  });

  // A poll-panel jump lands here with the poll to reveal. Declared last so it runs after the two
  // effects above and wins the final scroll position on a channel switch. Tracks the target rather
  // than the selection, so a jump to the channel you're already in scrolls too.
  $effect(() => {
    const target = ui.jump_poll;
    if (!target || !scroller) return;
    const el = scroller.querySelector(`[data-poll-anchor="${target}"]`);
    el?.scrollIntoView({ block: "center" });
    ui.jump_poll = null;
  });
</script>

<div class="h-full w-full bg-neutral-900 text-neutral-100">
  {#if ui.selected}
    <div class="flex h-full w-full flex-col">
      <header
        class="flex h-10 shrink-0 items-center gap-2 border-b border-neutral-800 px-3"
      >
        <span class="text-base font-semibold text-neutral-100">{header_name}</span
        >
        {#if archived}
          <span
            class="ml-1 rounded bg-neutral-800 px-1.5 py-0.5 text-xs text-neutral-400"
          >
            archived
          </span>
        {/if}

        <div class="ml-auto flex items-center gap-2">
          {#if notebook_borrowed}
            <span
              class="rounded bg-sky-600/20 px-2 py-0.5 text-xs font-medium text-sky-300"
              title="This notebook is currently on loan (being borrowed)."
            >
              Borrowed
            </span>
          {/if}

          {#if notebook_fake !== undefined}
            {@const cls = notebook_fake
              ? "bg-rose-600/20 text-rose-300"
              : "bg-emerald-600/20 text-emerald-300"}
            {#if is_admin}
              <button
                class="rounded px-2 py-0.5 text-xs font-medium hover:brightness-125 {cls}"
                title="Toggle whether this notebook is a decoy — a fake book's writes cannot kill."
                onclick={toggle_notebook_fake}
              >
                {notebook_fake ? "Fake" : "Real"}
              </button>
            {:else}
              <span
                class="rounded px-2 py-0.5 text-xs font-medium {cls}"
                title="A fake notebook's writes cannot kill. Only you and the host know this book's nature."
              >
                {notebook_fake ? "Fake" : "Real"}
              </span>
            {/if}
          {/if}

          {#if show_loggability}
            {@const cls = loggable
              ? "bg-amber-600/20 text-amber-400"
              : "bg-neutral-800 text-neutral-500"}
            {#if can_control_loggability}
              <button
                class="rounded px-2 py-0.5 text-xs font-medium hover:brightness-125 {cls}"
                title="Toggle whether messages sent here can be logged (autopsied / relayed to bugs)"
                onclick={toggle_loggable}
              >
                Logging {loggable ? "on" : "off"}
              </button>
            {:else}
              <span
                class="rounded px-2 py-0.5 text-xs font-medium {cls}"
                title="Whether messages sent here can be logged (autopsied / relayed to bugs)"
              >
                Logging {loggable ? "on" : "off"}
              </span>
            {/if}
          {/if}
        </div>
      </header>

      {#if ui.top_panel}
        <TopPanel />
      {/if}

      <main bind:this={scroller} class="min-h-0 flex-1 overflow-y-auto py-4">
        {#if start > 0}
          <div class="flex justify-center py-1">
            <button
              type="button"
              class="rounded bg-neutral-800 px-3 py-1 text-xs text-neutral-400 hover:bg-raised hover:text-neutral-200"
              onclick={load_earlier}
            >
              Load earlier messages
            </button>
          </div>
        {/if}
        {#each visible as event, i (event)}
          {@const gi = start + i}
          {#if "Message" in event.data}
            <Message
              senderDisplay={event.data.Message.sender_display}
              content={event.data.Message.content}
              {view}
              timestamp={event.timestamp}
              grouped={is_grouped_message(events[gi - 1], event)}
              last={!events[gi + 1] || !is_grouped_message(event, events[gi + 1])}
              mentioned={mentionsViewer(view, event.data.Message.content)}
            />
          {:else}
          <EventAnnouncement event={event} {view} timestamp={event.timestamp} />
          {/if}
        {/each}

        {#if is_news}
          {#if news_frozen}
            <div class="px-4 py-3 text-center text-xs text-amber-500/70">
              You are no longer receiving news. Everything above is what you
              last heard.
            </div>
          {/if}
          {#if !can_read}
            <div class="px-4 py-3 text-center text-xs text-neutral-500">
              You don't have access to this channel. Announcements above are
              game events and are always shown here — but you can't see chat
              messages.
            </div>
          {/if}
        {:else if !can_read}
          <div class="px-4 py-3 text-center text-xs text-neutral-500">
            You no longer have read access to this channel. Everything above is
            what you were given.
          </div>
        {/if}
        {#if archived}
          <div class="px-4 py-3 text-center text-xs text-neutral-500">
            This channel has been archived.
          </div>
        {/if}
      </main>

      <footer class="shrink-0 px-3 pb-2 pt-1">
        <div class="flex items-center gap-2">
          {#if can_send && sendable_profiles.length > 1}
            <select
              bind:value={selected_profile_key}
              class="bg-neutral-800 px-2 py-1.5 text-sm text-neutral-200"
            >
              {#each sendable_profiles as p (slotKeyToString(p.profile_id))}
                <option value={slotKeyToString(p.profile_id)}>
                  {actorDisplayLabel(p.display, view)}
                </option>
              {/each}
            </select>
          {/if}

          <div class="flex-1">
            {#if can_send}
              <div class="flex items-center gap-2 bg-neutral-800 px-2 py-1">
                <MentionInput
                  bind:value={message_content}
                  players={view.players}
                  orgs={view.orgs}
                  newsAnchor={view.news_anchor}
                  pressConf={view.press_conf}
                  statuses={view.actor_statuses}
                  placeholder={`Message ${channel_name ?? ""}`}
                  onsubmit={send_message}
                />
                <Button
                  size="sm"
                  onclick={async () => {
                    await send_message();
                  }}>Send</Button
                >
              </div>
              <!-- Frozen outranks the read-only blurbs: "this is no longer live" is the more
                 important of the two facts, and the blurb would otherwise hide it. -->
            {:else if frozen && read_only_feed}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                This feed no longer reaches you. Everything above is what it
                last relayed.
              </div>
            {:else if is_bug}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                {archived
                  ? "This surveillance feed is no longer active."
                  : "Read-only surveillance feed."}
              </div>
            {:else if is_contact_log}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                Read-only contact log. Names here are how each contact appeared,
                not who was really behind it.
              </div>
            {:else if is_log}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                {is_log_autopsy
                  ? "Autopsy — this actor's written record."
                  : "Tap-in — what was written down on the tapped line."}{" "}
                Read-only.
              </div>
            {:else if archived}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                This channel is archived and read-only.
              </div>
            {:else if frozen}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                You are no longer in this channel. Everything above is what you
                last saw.
              </div>
            {:else}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                You do not have permission to send messages in this channel.
              </div>
            {/if}
          </div>

          {#if notebook_id}
            <Button size="sm" variant="ghost" onclick={() => (pass_open = true)}
              >Pass</Button
            >
            <Button
              size="sm"
              variant="danger"
              onclick={() => (write_open = true)}>Write</Button
            >
          {/if}
        </div>
      </footer>

      {#if notebook_id}
        <NotebookWrite
          bind:open={write_open}
          notebookId={slotKeyFromString(notebook_id)}
        />
        <NotebookPass
          bind:open={pass_open}
          notebookId={slotKeyFromString(notebook_id)}
        />
      {/if}
    </div>
  {:else}
    <div class="flex h-full items-center justify-center text-neutral-500">
      Select a channel
    </div>
  {/if}
</div>
