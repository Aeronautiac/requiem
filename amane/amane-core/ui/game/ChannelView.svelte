<script lang="ts">
  import { getContext, tick } from "svelte";
  import MentionInput from "./MentionInput.svelte";
  import MentionText from "./MentionText.svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import {
    PERM_SEND,
    actorDisplayLabel,
    channelLabel,
    displayKey,
    isReadOnlyKind,
    mentionsViewer,
    nameLabel,
    orgColorVar,
    orgDisplayName,
    ownPerms,
    roleColorVar,
    roleLabel,
    t,
  } from "../../game/helpers.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { GameEvent, PollData, PollView, WriteEvent } from "../../game/types";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type {
    ActionRequest,
    PollOutcome,
    PollSubject,
    ProfileKey,
    ProsecutionPhaseView,
    TapInOutcome,
  } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { formatDuration } from "../../lib/utils";
  import Button from "../kit/Button.svelte";
  import Message from "./Message.svelte";
  import Announcement from "./Announcement.svelte";
  import Chip from "./Chip.svelte";
  import Name from "./Name.svelte";
  import ActorDisplay from "./ActorDisplay.svelte";
  import ContactLogRow from "./ContactLogRow.svelte";
  import TopPanel from "./TopPanel.svelte";
  import PollCard from "./PollCard.svelte";
  import PollNoticeCard from "./PollNoticeCard.svelte";
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

  // success = the name matched a real player; target_saved = the kill didn't land (write
  // immunity, or an earlier pending death on that target was cancelled by this write). The writer's
  // name renders as a coloured Name in the markup; this carries only the outcome lines below it.
  function write_outcome_text(w: WriteEvent): string {
    const lines: string[] = [];
    if (!w.success) {
      lines.push("Outcome: the name matched no one — no effect.");
    } else if (w.target_saved) {
      lines.push(
        "Outcome: valid name, but the target was saved (write immunity, or a pending death was cancelled).",
      );
    } else if (w.delay > 0) {
      lines.push(
        `Outcome: lethal — the target dies in ${formatDuration(w.delay)}.`,
      );
    } else {
      lines.push("Outcome: lethal — the target dies immediately.");
    }
    if (w.message) lines.push(`Note: ${w.message}`);
    lines.push(
      `Successes left: ${w.successes_remaining} · Attempts left: ${w.attempts_remaining}`,
    );
    return lines.join("\n");
  }

  // Red = lethal, amber = valid-but-saved, grey = no match.
  function write_event_color(w: WriteEvent): string {
    if (!w.success) return "var(--color-event-nothing)";
    if (w.target_saved) return "var(--color-event-alarm)";
    return "var(--color-event-death)";
  }

  // A miss says WHICH miss on purpose: a contact channel is loggable unless an admin turned it
  // off, so hitting a dark one is a real finding rather than a polite way of saying the number
  // was wrong.
  function tap_in_text(contact_id: number, outcome: TapInOutcome): string {
    if (outcome === "NoSuchContact") {
      return `Contact ${contact_id} does not exist. Nobody has ever been connected under that number.`;
    }
    if (outcome === "NotLoggable") {
      return `Contact ${contact_id} exists, but logging is off there — nothing was ever written down.`;
    }
    const scope =
      outcome.Found.range === null
        ? "everything it has ever carried"
        : `the last ${formatDuration(outcome.Found.range)}`;
    return `Tapped into contact ${contact_id}. Reading ${scope}.`;
  }

  // The third death beat, when something changed hands. Never names who received it — that is the
  // mystery the reveal is built around.
  function death_transfer_text(tr: {
    notebook_transferred: boolean;
    ability_transferred: boolean;
  }): string {
    if (tr.notebook_transferred && tr.ability_transferred) {
      return "their notebook and their power have";
    }
    return tr.notebook_transferred ? "their notebook has" : "their power has";
  }

  // A poll's start notice rides its home channel's stream, so its position in the log IS where the
  // poll belongs chronologically. While the poll is still live we render the interactive card in
  // place of the "vote started" announcement; once it resolves, the announcement (with its outcome)
  // takes over again. Absent/resolved polls answer null and fall through to the announcement.
  function live_poll(
    poll_id: string,
  ): { data: PollData; pollView: PollView | null; frozen: boolean } | null {
    const data = view.polls.get(poll_id);
    if (!data || data.outcome) return null;
    return {
      data,
      pollView: view.poll_views.get(poll_id) ?? null,
      frozen: view.frozen(view.poll_viewport(poll_id)),
    };
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
          {:else if "Write" in event.data}
            {@const w = event.data.Write}
            <Announcement {view} timestamp={event.timestamp} color={write_event_color(w)} description="Notebook Write">
              <Name id={w.user_id} {view} chip /> wrote the name
              <span class="text-neutral-200">"{nameLabel(w.true_name)}"</span>.
              <div class="mt-1 whitespace-pre-wrap">{write_outcome_text(w)}</div>
            </Announcement>
          {:else if "Death" in event.data}
            {@const d = event.data.Death}
            <!-- Beat 1 of the staged reveal: the death and the name behind it. The role and any
                 inheritance follow as their own events (DeathRole / DeathTransfer), timed apart. -->
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-death)" description="Death">
              <Name id={d.target_id} {view} chip /> has died.
              {#if d.death_message}
                <div class="mt-1 italic whitespace-pre-wrap text-neutral-300">
                  <MentionText content={d.death_message} {view} />
                </div>
              {/if}
              <div class="mt-1 text-neutral-400">
                Their true name was <span class="text-neutral-200"
                  >{nameLabel(d.true_name)}</span
                >.
              </div>
            </Announcement>
          {:else if "DeathRole" in event.data}
            {@const r = event.data.DeathRole}
            <!-- Beat 2: who they turned out to be, in that role's own colour. -->
            <Announcement {view} timestamp={event.timestamp} color={roleColorVar(r.role)} description="Role Revealed">
              <Name id={r.target_id} {view} chip /> was
              <Chip label={roleLabel(r.role)} colorVar={roleColorVar(r.role)} />.
            </Announcement>
          {:else if "DeathOrgs" in event.data}
            {@const o = event.data.DeathOrgs}
            <!-- Beat: the affiliations they turn out to have had. On a real death these are true; on
                 a pseudocide they are whatever the faker chose to show. Leadership and OG standing,
                 normally private, are laid bare here. -->
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-reveal)" description="Affiliations">
              <Name id={o.target_id} {view} chip /> stood with
              {#each o.orgs as org, i (org.id)}
                {@const name = view.orgs.get(org.id)?.name}
                <Chip
                  label={name ? orgDisplayName(name) : t("display_org_unknown")}
                  colorVar={name ? orgColorVar(name) : "var(--color-event-reveal)"}
                />{#if org.leader}<span class="text-neutral-400"> (leader)</span>{:else if org.og}<span class="text-neutral-400"> (OG)</span>{/if}{#if i < o.orgs.length - 1}, {/if}
              {/each}.
            </Announcement>
          {:else if "DeathTransfer" in event.data}
            {@const tr = event.data.DeathTransfer}
            <!-- Beat 3: what they left behind — never to whom. -->
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-death)" description="Inheritance">
              <Name id={tr.target_id} {view} chip /> is gone — but {death_transfer_text(tr)} passed
              to someone new.
            </Announcement>
          {:else if "AnonymousAnnouncement" in event.data}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-anonymous)"
              description="Anonymous Announcement"
            >
              <div class="whitespace-pre-wrap">
                <MentionText content={event.data.AnonymousAnnouncement.content} {view} />
              </div>
            </Announcement>
          {:else if "EyeDealTaken" in event.data}
            {@const u = event.data.EyeDealTaken.user}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-reveal)" description="The Eye Deal">
              <ActorDisplay display={u} {view} /> has taken the shinigami eye
              deal.
            </Announcement>
          {:else if "NewsAnchor" in event.data}
            {@const na = event.data.NewsAnchor}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-news-anchor)" description="News Anchor">
              {#if na.target_id}
                <Name id={na.target_id} {view} chip /> is now the
                <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
              {:else}
                The <Chip label="News Anchor" colorVar="var(--color-news-anchor)" /> post
                is now vacant.
              {/if}
            </Announcement>
          {:else if "NewsAnchorStatus" in event.data}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-personal)" description="News Anchor">
              {#if event.data.NewsAnchorStatus.holding}
                You are now the
                <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
              {:else}
                You are no longer the
                <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
              {/if}
            </Announcement>
          {:else if "PressConfMembership" in event.data}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-personal)"
              description="Press Conference"
            >
              {#if event.data.PressConfMembership.in_conf}
                You joined the
                <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
              {:else}
                You left the
                <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
              {/if}
            </Announcement>
          {:else if "LeaderStatus" in event.data}
            {@const ls = event.data.LeaderStatus}
            {@const org_name = view.orgs.get(ls.org_id)?.name}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-personal)" description="Leadership">
              {#if ls.leader}You are now the leader of{:else}You are no longer the leader of{/if}
              <Chip
                label={org_name ? orgDisplayName(org_name) : t("display_org_unknown")}
                colorVar={org_name ? orgColorVar(org_name) : "var(--color-event-personal)"}
              />.
            </Announcement>
          {:else if "PressConfStatus" in event.data}
            {@const pc = event.data.PressConfStatus}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-press-conference)"
              description="Press Conference"
            >
              <Name id={pc.target_id} {view} chip />
              {pc.has_access ? "joined the" : "left the"}
              <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
            </Announcement>
          {:else if "FailedSilentProsecution" in event.data}
            {@const f = event.data.FailedSilentProsecution}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-prosecution)"
              description="False Accusation"
            >
              <Name id={f.accuser_id} {view} chip /> named an innocent person as wanted.
              <div class="mt-1">
                Real name: <span class="text-neutral-200">{nameLabel(f.true_name)}</span>
              </div>
              <div class="mt-1">
                {orgDisplayName(f.org)} has expelled them and barred them from returning.
              </div>
            </Announcement>
          {:else if "RevealTrueName" in event.data}
            {@const r = event.data.RevealTrueName}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-reveal)" description="Name Reveal">
              <Name id={r.target_id} {view} chip />'s true name is
              <span class="text-neutral-200">{nameLabel(r.true_name)}</span>.
            </Announcement>
          {:else if "RevealNotebookHolding" in event.data}
            {@const r = event.data.RevealNotebookHolding}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-reveal)" description="Notebook Check">
              <Name id={r.target_id} {view} chip /> is {r.holding ? "" : "not "}currently
              holding a notebook.
            </Announcement>
          {:else if "EyeCount" in event.data}
            {@const c = event.data.EyeCount.count}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-personal)" description="Shinigami Eyes">
              You have <span class="text-neutral-200">{c}</span> eye{c === 1 ? "" : "s"} remaining.
            </Announcement>
          {:else if "Bugged" in event.data}
            {@const b = event.data.Bugged}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-surveillance)"
              description="Surveillance"
              content={b.context === "Custody"
                ? "You are bugged: your messages are being monitored while you are in custody."
                : "You have been bugged. Your messages are being monitored."}
            />
          {:else if "RoleUpdate" in event.data}
            {@const role = event.data.RoleUpdate.role}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-personal)" description="Role">
              Your role is now <Chip
                label={roleLabel(role)}
                colorVar={roleColorVar(role)}
              />.
            </Announcement>
          {:else if "TrueNameUpdate" in event.data}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-personal)"
              description="True Name"
              content={`Your true name is now ${nameLabel(event.data.TrueNameUpdate.true_name)}.`}
            />
          {:else if "NotebookReceived" in event.data}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-death)"
              description="Notebook"
              content="A notebook has come into your possession."
            />
          {:else if "PollNotice" in event.data}
            {@const pn = event.data.PollNotice}
            {@const live = pn.outcome ? null : live_poll(pn.poll_id)}
            {#if live}
              <div class="px-3 py-1" data-poll-anchor={pn.poll_id}>
                <PollCard
                  id={pn.poll_id}
                  data={live.data}
                  pollView={live.pollView}
                  frozen={live.frozen}
                  variant="inline"
                />
              </div>
            {:else}
              <PollNoticeCard
                poll_id={pn.poll_id}
                subject={pn.subject}
                outcome={pn.outcome}
                opener={pn.opener}
                timestamp={event.timestamp}
              />
            {/if}
          {:else if "PseudocideRevival" in event.data}
            {@const r = event.data.PseudocideRevival}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-revival)" description="Revival">
              <Name id={r.target_id} {view} chip /> is alive.
            </Announcement>
          {:else if "KidnapReveal" in event.data}
            {@const kr = event.data.KidnapReveal}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-alarm)" description="Kidnap Reveal">
              Authorities have recovered {#if kr.victim}<Name
                  id={kr.victim}
                  {view}
                  chip
                />{:else}the victim{/if}{#if kr.kidnapper}, and <Name
                  id={kr.kidnapper}
                  {view}
                  chip
                /> was revealed as the kidnapper.{:else}, but the kidnapper stayed anonymous.{/if}
            </Announcement>
          {:else if "Kidnapping" in event.data}
            {@const k = event.data.Kidnapping}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-alarm)" description="Kidnapping">
              <Name id={k.target_id} {view} chip /> has been kidnapped.
            </Announcement>
          {:else if "Incarceration" in event.data}
            {@const inc = event.data.Incarceration}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-custody)" description="Imprisonment">
              <Name id={inc.victim_id} {view} chip /> has been imprisoned{#if inc.duration}
                for {formatDuration(inc.duration)}{/if}.
            </Announcement>
          {:else if "IncarcerationReleased" in event.data}
            {@const rel = event.data.IncarcerationReleased}
            <Announcement {view} timestamp={event.timestamp} color="var(--color-event-custody)" description="Release">
              {#if rel.victim}<Name id={rel.victim} {view} chip />{:else}A prisoner{/if} has
              been released.
            </Announcement>
          {:else if "NewIteration" in event.data}
            {@const it = event.data.NewIteration.iteration}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-alarm)"
              description={it === 1 ? "The Game Begins" : "New Day"}
              content={it === 1
                ? "Day 1. Abilities and notebooks are live."
                : `Day ${it}.`}
            />
          {:else if "Blackout" in event.data}
            {@const on = event.data.Blackout.active}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-blackout)"
              description={on
                ? t("blackout_begun_label")
                : t("blackout_over_label")}
              content={on ? t("blackout_begun") : t("blackout_over")}
            />
          {:else if "ChannelTapped" in event.data}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-surveillance)"
              description="Tapped"
              content="Someone outside this conversation read what was said here. There is no way to tell who."
            />
          {:else if "TapInResult" in event.data}
            {@const tr = event.data.TapInResult}
            <Announcement {view} timestamp={event.timestamp}
              color={typeof tr.outcome === "string"
                ? "var(--color-event-nothing)"
                : "var(--color-event-tap)"}
              description="Tap In"
              content={tap_in_text(tr.contact_id, tr.outcome)}
            />
          {:else if "FakeLoungeTapped" in event.data}
            {@const fl = event.data.FakeLoungeTapped}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-surveillance)"
              description="Fake Lounge Read"
            >
              Your fabricated lounge was read by
              <ActorDisplay display={fl.display} {view} />.
            </Announcement>
          {:else if "KiraConnectionAttempt" in event.data}
            {@const ka = event.data.KiraConnectionAttempt}
            <Announcement {view} timestamp={event.timestamp}
              color={ka.success
                ? "var(--color-event-death)"
                : "var(--color-event-nothing)"}
              description="Kira Connection"
            >
              <Name id={ka.user} {view} chip /> reached for Kira through this line{ka.success
                ? " — and Kira answered."
                : ". Nobody answered."}
            </Announcement>
          {:else if "ContactLogEntry" in event.data}
            {@const log = event.data.ContactLogEntry}
            <ContactLogRow
              from={log.contactor}
              to={log.contacted}
              event={log.event}
              timestamp={event.timestamp}
              {view}
            />
          {:else if "ProsecutionEvent" in event.data}
            {@const pe = event.data.ProsecutionEvent}
            {#snippet defendant()}<ActorDisplay display={pe.defendant_display} {view} />{/snippet}
            {#snippet prosecutor()}<ActorDisplay display={pe.prosecutor_display} {view} />{/snippet}
            <Announcement {view} timestamp={event.timestamp}
              color="var(--color-event-prosecution)"
              description={pe.ended ? "Prosecution Ended" : "Prosecution"}
            >
              {#if pe.ended}
                {#if pe.verdict === true}
                  {@render defendant()} has been found guilty.
                {:else if pe.verdict === false}
                  {@render defendant()} has been acquitted.
                {:else}
                  The prosecution of {@render defendant()} has ended.
                {/if}
              {:else if pe.phase === "Voting"}
                The trial vote for {@render defendant()} has begun.
              {:else if "Custody" in pe.phase}
                {@render prosecutor()} is prosecuting {@render defendant()}.
              {:else if "Debate" in pe.phase.Trial}
                The trial of {@render defendant()} has entered debate.
              {:else if "Prosecutor" in pe.phase.Trial}
                {#if pe.phase.Trial.Prosecutor === "Grace"}
                  The trial of {@render defendant()} has begun, the prosecution has the floor.
                {:else}
                  In the trial of {@render defendant()}, the prosecution presents.
                {/if}
              {:else}
                {#if pe.phase.Trial.Defense === "Grace"}
                  In the trial of {@render defendant()}, the defense has the floor.
                {:else}
                  In the trial of {@render defendant()}, the defense presents.
                {/if}
              {/if}
            </Announcement>
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
