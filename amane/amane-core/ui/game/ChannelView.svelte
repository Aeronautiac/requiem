<script lang="ts">
  import { getContext } from "svelte";
  import MentionInput from "./MentionInput.svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import {
    PERM_SEND,
    actorLabel,
    channelLabel,
    displayKey,
    isReadOnlyKind,
    mentionsViewer,
    nameLabel,
    orgDisplayName,
    ownPerms,
    phaseAnnouncement,
    playerLabel,
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
    ActorDisplay,
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
  import ContactLogRow from "./ContactLogRow.svelte";
  import TopPanel from "./TopPanel.svelte";
  import PollCard from "./PollCard.svelte";
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

  function player_name(id: string): string {
    return playerLabel(id, view.players);
  }

  function poll_notice_text(subject: PollSubject): string {
    if ("Generic" in subject) return subject.Generic;
    if ("CivilianArrest" in subject) {
      return `Arrest ${playerLabel(slotKeyToString(subject.CivilianArrest), view.players)}`;
    }
    const beh = subject.OrgAbility as Record<string, unknown>;
    const name = Object.keys(beh)[0] ?? "";
    return name.replace(/([a-z])([A-Z])/g, "$1 $2");
  }

  // A resolved poll names the option that won, so the label comes from the poll itself. The entry
  // is never deleted, so it is still there however long ago the vote closed.
  function poll_outcome_text(poll_id: string, outcome: PollOutcome): string {
    if (outcome === "Inconclusive" || outcome === "Cancelled") return outcome;
    const label = view.polls.get(poll_id)?.options[outcome.Resolved]?.label;
    if (label === undefined) return "resolved";
    return typeof label === "string" ? label : label.Generic;
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

  function display_string(display: ActorDisplay): string {
    return actorLabel(display, view.players);
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
  // immunity, or an earlier pending death on that target was cancelled by this write).
  function write_event_text(w: WriteEvent): string {
    const lines = [
      `${player_name(w.user_id)} wrote the name "${nameLabel(w.true_name)}".`,
    ];
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

  function prosecution_event_text(pe: {
    prosecutor_display: ActorDisplay;
    defendant_display: ActorDisplay;
    phase: ProsecutionPhaseView;
    ended: boolean;
    verdict: boolean | null;
  }): string {
    return phaseAnnouncement(
      pe.phase,
      display_string(pe.prosecutor_display),
      display_string(pe.defendant_display),
      pe.ended,
      pe.verdict,
    );
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
        {#each events as event, i (event)}
          {#if "Message" in event.data}
            <Message
              sender={display_string(event.data.Message.sender_display)}
              content={event.data.Message.content}
              players={view.players}
              timestamp={event.timestamp}
              grouped={is_grouped_message(events[i - 1], event)}
              last={!events[i + 1] || !is_grouped_message(event, events[i + 1])}
              mentioned={mentionsViewer(view, event.data.Message.content)}
            />
          {:else if "Write" in event.data}
            {@const w = event.data.Write}
            <Announcement
              color={write_event_color(w)}
              description="Notebook Write"
              content={write_event_text(w)}
            />
          {:else if "Death" in event.data}
            {@const d = event.data.Death}
            <!-- Beat 1 of the staged reveal: the death and the name behind it. The role and any
                 inheritance follow as their own events (DeathRole / DeathTransfer), timed apart. -->
            <Announcement color="var(--color-event-death)" description="Death">
              <Chip
                label={player_name(d.target_id)}
                colorVar="var(--color-mention-player)"
              /> has died.
              {#if d.death_message}
                <div class="mt-1 italic text-neutral-300">{d.death_message}</div>
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
            <Announcement color={roleColorVar(r.role)} description="Role Revealed">
              <Chip
                label={player_name(r.target_id)}
                colorVar="var(--color-mention-player)"
              /> was
              <Chip label={roleLabel(r.role)} colorVar={roleColorVar(r.role)} />.
            </Announcement>
          {:else if "DeathTransfer" in event.data}
            {@const tr = event.data.DeathTransfer}
            <!-- Beat 3: what they left behind — never to whom. -->
            <Announcement color="var(--color-event-death)" description="Inheritance">
              <Chip
                label={player_name(tr.target_id)}
                colorVar="var(--color-mention-player)"
              /> is gone — but {death_transfer_text(tr)} passed to someone new.
            </Announcement>
          {:else if "AnonymousAnnouncement" in event.data}
            <Announcement
              color="var(--color-event-anonymous)"
              description="Anonymous Announcement"
              content={event.data.AnonymousAnnouncement.content}
            />
          {:else if "FailedSilentProsecution" in event.data}
            {@const f = event.data.FailedSilentProsecution}
            <Announcement
              color="var(--color-event-prosecution)"
              description="False Accusation"
              content={`${player_name(f.accuser_id)} named an innocent person as wanted.\nReal name: ${nameLabel(f.true_name)}\n\n${orgDisplayName(f.org)} has expelled them and barred them from returning.`}
            />
          {:else if "RevealTrueName" in event.data}
            {@const r = event.data.RevealTrueName}
            <Announcement
              color="var(--color-event-reveal)"
              description="Name Reveal"
              content={`${player_name(r.target_id)}'s true name is ${nameLabel(r.true_name)}.`}
            />
          {:else if "RevealNotebookHolding" in event.data}
            {@const r = event.data.RevealNotebookHolding}
            <Announcement
              color="var(--color-event-reveal)"
              description="Notebook Check"
              content={`${player_name(r.target_id)} is ${r.holding ? "" : "not "}currently holding a notebook.`}
            />
          {:else if "Bugged" in event.data}
            {@const b = event.data.Bugged}
            <Announcement
              color="var(--color-event-surveillance)"
              description="Surveillance"
              content={b.context === "Custody"
                ? "You are bugged: your messages are being monitored while you are in custody."
                : "You have been bugged. Your messages are being monitored."}
            />
          {:else if "RoleUpdate" in event.data}
            {@const role = event.data.RoleUpdate.role}
            <Announcement color="var(--color-event-personal)" description="Role">
              Your role is now <Chip
                label={roleLabel(role)}
                colorVar={roleColorVar(role)}
              />.
            </Announcement>
          {:else if "TrueNameUpdate" in event.data}
            <Announcement
              color="var(--color-event-personal)"
              description="True Name"
              content={`Your true name is now ${nameLabel(event.data.TrueNameUpdate.true_name)}.`}
            />
          {:else if "NotebookReceived" in event.data}
            <Announcement
              color="var(--color-event-death)"
              description="Notebook"
              content="A notebook has come into your possession."
            />
          {:else if "PollNotice" in event.data}
            {@const pn = event.data.PollNotice}
            {@const live = pn.outcome ? null : live_poll(pn.poll_id)}
            {#if live}
              <div class="px-3 py-1">
                <PollCard
                  id={pn.poll_id}
                  data={live.data}
                  pollView={live.pollView}
                  frozen={live.frozen}
                  variant="inline"
                />
              </div>
            {:else}
              <Announcement
                color="var(--color-event-vote)"
                description={pn.outcome
                  ? `Vote: ${poll_outcome_text(pn.poll_id, pn.outcome)}`
                  : "Vote started"}
                content={pn.opener
                  ? `${poll_notice_text(pn.subject)}\nStarted by ${view.actor_name(pn.opener)}`
                  : poll_notice_text(pn.subject)}
              />
            {/if}
          {:else if "PseudocideRevival" in event.data}
            {@const r = event.data.PseudocideRevival}
            <Announcement
              color="var(--color-event-revival)"
              description="Revival"
              content={`${player_name(r.target_id)} is alive.`}
            />
          {:else if "KidnapReveal" in event.data}
            {@const kr = event.data.KidnapReveal}
            {@const victim = kr.victim ? player_name(kr.victim) : "the victim"}
            <Announcement
              color="var(--color-event-alarm)"
              description="Kidnap Reveal"
              content={kr.kidnapper
                ? `Authorities have recovered ${victim}, and ${player_name(kr.kidnapper)} was revealed as the kidnapper.`
                : `Authorities have recovered ${victim}, but the kidnapper stayed anonymous.`}
            />
          {:else if "Kidnapping" in event.data}
            {@const k = event.data.Kidnapping}
            <Announcement
              color="var(--color-event-alarm)"
              description="Kidnapping"
              content={`${player_name(k.target_id)} has been kidnapped.`}
            />
          {:else if "Incarceration" in event.data}
            {@const inc = event.data.Incarceration}
            <Announcement
              color="var(--color-event-custody)"
              description="Imprisonment"
              content={inc.duration
                ? `${player_name(inc.victim_id)} has been imprisoned for ${formatDuration(inc.duration)}.`
                : `${player_name(inc.victim_id)} has been imprisoned.`}
            />
          {:else if "IncarcerationReleased" in event.data}
            {@const rel = event.data.IncarcerationReleased}
            <Announcement
              color="var(--color-event-custody)"
              description="Release"
              content={`${rel.victim ? player_name(rel.victim) : "A prisoner"} has been released.`}
            />
          {:else if "NewIteration" in event.data}
            {@const it = event.data.NewIteration.iteration}
            <Announcement
              color="var(--color-event-alarm)"
              description={it === 1 ? "The Game Begins" : "New Day"}
              content={it === 1
                ? "Day 1. Abilities and notebooks are live."
                : `Day ${it}.`}
            />
          {:else if "Blackout" in event.data}
            {@const on = event.data.Blackout.active}
            <Announcement
              color="var(--color-event-blackout)"
              description={on
                ? t("blackout_begun_label")
                : t("blackout_over_label")}
              content={on ? t("blackout_begun") : t("blackout_over")}
            />
          {:else if "ChannelTapped" in event.data}
            <Announcement
              color="var(--color-event-surveillance)"
              description="Tapped"
              content="Someone outside this conversation read what was said here. There is no way to tell who."
            />
          {:else if "TapInResult" in event.data}
            {@const tr = event.data.TapInResult}
            <Announcement
              color={typeof tr.outcome === "string"
                ? "var(--color-event-nothing)"
                : "var(--color-event-tap)"}
              description="Tap In"
              content={tap_in_text(tr.contact_id, tr.outcome)}
            />
          {:else if "KiraConnectionAttempt" in event.data}
            {@const ka = event.data.KiraConnectionAttempt}
            <Announcement
              color={ka.success
                ? "var(--color-event-death)"
                : "var(--color-event-nothing)"}
              description="Kira Connection"
              content={ka.success
                ? `${player_name(ka.user)} reached for Kira through this line — and Kira answered.`
                : `${player_name(ka.user)} reached for Kira through this line. Nobody answered.`}
            />
          {:else if "ContactLogEntry" in event.data}
            {@const log = event.data.ContactLogEntry}
            <ContactLogRow
              from={display_string(log.contactor)}
              to={display_string(log.contacted)}
              event={log.event}
              timestamp={event.timestamp}
            />
          {:else if "ProsecutionEvent" in event.data}
            {@const pe = event.data.ProsecutionEvent}
            <Announcement
              color="var(--color-event-prosecution)"
              description={pe.ended ? "Prosecution Ended" : "Prosecution"}
              content={prosecution_event_text(pe)}
            />
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
                  {display_string(p.display)}
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
