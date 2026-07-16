<script lang="ts">
  import { getContext } from "svelte";
  import Input from "$lib/components/ui/input/input.svelte";
  import { GAME_STATE_KEY, displayKey } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameEvent, GameState, WriteEvent } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { ActionRequest, ActorDisplay, PollSubject, ProsecutionPhaseView } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { formatDuration } from "$lib/utils";
  import Button from "$lib/components/ui/button/button.svelte";
  import Message from "./Message.svelte";
  import Announcement from "./Announcement.svelte";
  import NotebookWrite from "./NotebookWrite.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  let message_content = $state("");
  let channel_name = $derived(get_channel_name());
  let scroller = $state<HTMLElement>();

  // Admin implicitly has every permission; players go by their channel_perms entry
  // (which may be absent, meaning no access).
  const is_admin = $derived(ui.viewer === "Admin");

  // News is its own selection, not a channel — it always exists on the frontend and
  // carries no channel data. Its backing channel (for messages and perms) is
  // news_channel_id, which may be absent; that's only a lookup, never news's identity.
  // Every other channel is keyed directly by selected_channel. World events render
  // regardless (see the events derived) since they live per-view, not on the channel.
  const is_news = $derived(ui.is_news);
  const backing_channel_id = $derived(
    is_news ? game.news_channel_id : ui.selected_channel,
  );

  const current_channel = $derived(
    backing_channel_id
      ? game.resolve_channel(ui.viewer, backing_channel_id)
      : undefined,
  );
  const current_perms = $derived(
    backing_channel_id
      ? game.views.get(ui.viewer)?.channel_views.get(backing_channel_id)?.perms
      : undefined,
  );
  // Info channels are frontend-only, read-only feeds (name reveals, autopsies): you
  // can never send in one, and their owner can always read them (no engine perms).
  const is_info = $derived(current_channel?.kind === "Info");
  // Bug feeds are read-only surveillance logs: never interactable, and always readable to
  // whoever the feed was listed for (visibility is gated when it's shown in the sidebar).
  const is_bug = $derived(current_channel?.kind === "Bug");
  const archived = $derived(current_channel?.archived ?? false);
  // You can only send in a channel that actually exists. For news with no backing
  // channel this is false, so it isn't interactable — only the event log shows.
  // Archived channels are read-only for everyone, admin included.
  const can_send = $derived(
    current_channel != null &&
      !archived &&
      !is_info &&
      !is_bug &&
      (is_admin || (current_perms?.send ?? false)),
  );
  const can_read = $derived(is_info || is_bug || is_admin || (current_perms?.read ?? false));
  // Notebook-ness isn't a channel kind — it's derived from the channel->notebook mapping.
  // A non-undefined notebook_id both identifies the channel as a notebook and gives the
  // Write affordance its target.
  const notebook_id = $derived(
    backing_channel_id
      ? game.notebook_for_channel(backing_channel_id)
      : undefined,
  );
  // Loggability is a global channel property (SetChannelLoggable). Show it, and let a viewer
  // with loggability control flip it. Synthetic info/bug feeds aren't engine channels, so the
  // control never applies there.
  const loggable = $derived(
    backing_channel_id ? game.is_channel_loggable(backing_channel_id) : false,
  );
  // Show the status on any real engine channel (info/bug feeds aren't engine channels);
  // it becomes an interactive toggle only for viewers with loggability control.
  const show_loggability = $derived(current_channel != null && !is_info && !is_bug);
  const can_control_loggability = $derived(
    show_loggability && (is_admin || (current_perms?.loggability_control ?? false)),
  );
  let write_open = $state(false);

  // The displays the viewer may send as in the current channel (their "send as"
  // options, delivered by UpdateChannelView). Empty for admin (always sends as System).
  const available_displays = $derived(
    backing_channel_id
      ? (game.views.get(ui.viewer)?.channel_views.get(backing_channel_id)
          ?.displays ?? [])
      : [],
  );
  let selected_display_key = $state<string | null>(null);

  // Keep the selection valid as the channel (and thus the options) changes: if the
  // current pick isn't available, fall back to the first option.
  $effect(() => {
    const keys = available_displays.map(displayKey);
    if (!selected_display_key || !keys.includes(selected_display_key)) {
      selected_display_key = keys[0] ?? null;
    }
  });

  // Admin reads the System view's world events; players read their own.
  // TODO: engine must emit world events to System for this to be populated.
  const current_view = $derived(
    is_admin ? game.system_view() : game.views.get(ui.viewer),
  );
  // News may be selected without a backing channel object, so fall back to a name.
  const header_name = $derived(channel_name ?? (is_news ? "News" : ""));

  function get_channel_name(): string | null {
    return backing_channel_id
      ? (game.resolve_channel(ui.viewer, backing_channel_id)?.name ?? null)
      : null;
  }

  function player_name(id: string): string {
    return game.players.get(id)?.display_name ?? "Unknown";
  }

  // Short label for a poll notice rendered in-channel (the Polls panel is where you vote).
  function poll_notice_text(subject: PollSubject): string {
    if ("Generic" in subject) return subject.Generic;
    if ("CivilianArrest" in subject) {
      const nm = game.players.get(slotKeyToString(subject.CivilianArrest))?.display_name;
      return nm ? `Arrest ${nm}` : "Civilian arrest";
    }
    const beh = subject.OrgAbility as Record<string, unknown>;
    const name = Object.keys(beh)[0] ?? "";
    return name.replace(/([a-z])([A-Z])/g, "$1 $2");
  }

  function sender_display(): ActorDisplay {
    if (ui.viewer === "Admin") return "System";
    const chosen = available_displays.find(
      (d) => displayKey(d) === selected_display_key,
    );
    return chosen ?? { Raw: slotKeyFromString(ui.viewer) };
  }

  function display_string(display: ActorDisplay): string {
    if (display === "Mysterious") return "???";
    if (display === "System") return "System";
    if ("Raw" in display)
      return (
        game.players.get(slotKeyToString(display.Raw))?.display_name ??
        "Unknown"
      );
    if ("Role" in display) return display.Role;
    if ("Org" in display) return "Org";
    return "Unknown";
  }

  // Discord-style chunking: a message is a "continuation" (its sender header is dropped)
  // when the immediately preceding rendered event is ALSO a message, from the same display,
  // within this window. Any non-message event in between breaks the chain — the run must be
  // uninterrupted. Each message stays its own row (own hover, own timestamp); only the header
  // is suppressed.
  const GROUP_WINDOW_MS = 45_000;
  function is_grouped_message(
    prev: GameEvent | undefined,
    curr: GameEvent,
  ): boolean {
    if (!prev || !("Message" in prev.data) || !("Message" in curr.data)) return false;
    if (
      displayKey(prev.data.Message.sender_display) !==
      displayKey(curr.data.Message.sender_display)
    )
      return false;
    return curr.timestamp - prev.timestamp <= GROUP_WINDOW_MS;
  }

  // A notebook write, rendered with everything a viewer holding the book needs: who wrote,
  // the name written, the outcome (lethal / target saved / no match), any death note, and
  // the writer's remaining successes/attempts. success=true means the name matched a real
  // player; target_saved means the kill didn't land (write immunity, or an earlier pending
  // death on that target was cancelled by this write).
  function write_event_text(w: WriteEvent): string {
    const lines = [`${player_name(w.user_id)} wrote the name "${w.true_name}".`];
    if (!w.success) {
      lines.push("Outcome: the name matched no one — no effect.");
    } else if (w.target_saved) {
      lines.push(
        "Outcome: valid name, but the target was saved (write immunity, or a pending death was cancelled).",
      );
    } else if (w.delay > 0) {
      lines.push(`Outcome: lethal — the target dies in ${formatDuration(w.delay)}.`);
    } else {
      lines.push("Outcome: lethal — the target dies immediately.");
    }
    if (w.message) lines.push(`Note: ${w.message}`);
    lines.push(
      `Successes left: ${w.successes_remaining} · Attempts left: ${w.attempts_remaining}`,
    );
    return lines.join("\n");
  }

  // Colour by outcome: red = lethal, amber = valid-but-saved, grey = no match.
  function write_event_color(w: WriteEvent): string {
    if (!w.success) return "#6b7280";
    if (w.target_saved) return "#f59e0b";
    return "#ef4444";
  }

  // News-feed text for a prosecution start/advance/end (derived from the phase diff).
  function prosecution_event_text(pe: {
    prosecutor_display: ActorDisplay;
    defendant_display: ActorDisplay;
    phase: ProsecutionPhaseView;
    ended: boolean;
  }): string {
    const prosecutor = display_string(pe.prosecutor_display);
    const defendant = display_string(pe.defendant_display);
    if (pe.ended) return `The prosecution of ${defendant} has ended.`;
    const p = pe.phase;
    if (p === "Custody") return `${prosecutor} is prosecuting ${defendant}.`;
    if (p === "Voting") return `The verdict vote for ${defendant} has begun.`;
    if (p.Trial === "Prosecutor")
      return `The trial of ${defendant} has begun — the prosecution presents.`;
    if (p.Trial === "Defense")
      return `In the trial of ${defendant}, the defense presents.`;
    return `The trial of ${defendant} has entered debate.`;
  }

  async function send_message() {
    // backing_channel_id resolves news to its real channel key; can_send already
    // gates the box on the channel existing, so this is null only defensively.
    if (!backing_channel_id || !message_content.trim()) return;
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload: {
        SendMessage: {
          channel_id: slotKeyFromString(backing_channel_id),
          display: sender_display(),
          content: message_content.trim(),
        },
      },
    };
    await game.dispatch(request);
    message_content = "";
    console.log("message sent");
  }

  async function toggle_loggable() {
    if (!backing_channel_id) return;
    await game.dispatch({
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

  // based on the context, choose which events to include, push them into a unified list, and then sort the list
  const events: GameEvent[] = $derived.by(() => {
    // Nothing selected → nothing to show. News (which has no channel key) still
    // counts as selected, so gate on the selection itself, not the channel key.
    if (!ui.selected) {
      return [];
    }

    let accum: GameEvent[] = [];

    // News always shows the viewer's world events (deaths, anonymous
    // announcements). These are game events, not channel messages, so they render
    // regardless of whether the news channel currently exists or the viewer has
    // any perms for it.
    if (is_news && current_view) {
      accum = accum.concat(current_view.events);
    }

    // Channel messages/writes follow the normal perm rules. The channel object may
    // be absent — a channel removed while still selected, or news that doesn't
    // currently exist — in which case there are simply no messages to add.
    const channel = current_channel;
    if (channel) {
      // Full history if admin or currently readable; if the viewer was once a real
      // member (had_positive) but read is now off, only messages up to the cutoff;
      // otherwise (never a member, e.g. L & Watari) → nothing.
      if (can_read) {
        accum = accum.concat(channel.events);
      } else if (current_perms && current_perms.had_positive) {
        accum = accum.concat(
          channel.events.filter(
            (val) => val.timestamp <= current_perms.read_updated,
          ),
        );
      }
    }

    // oldest to newest
    accum.sort((a, b) => a.timestamp - b.timestamp);

    return accum;
  });

  // Switching channels always lands at the bottom. The <main> element is reused
  // across channels, so its scrollTop would otherwise carry over from the last one.
  $effect(() => {
    ui.selected; // track: re-run when the selected channel/news changes
    scroller?.scrollTo({ top: scroller.scrollHeight });
  });

  // New messages in the current channel stick to the bottom, but only if the user
  // is already near it (so scrolling up to read history isn't yanked back down).
  // Runs after the DOM flush, so scrollHeight already includes the new content.
  $effect(() => {
    events.length; // track: re-run whenever the event list grows
    if (!scroller) return;
    const near_bottom =
      scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 100;
    if (near_bottom) scroller.scrollTo({ top: scroller.scrollHeight });
  });
</script>

<div class="h-full w-full bg-neutral-900 text-neutral-100">
  {#if ui.selected}
    <div class="grid h-full w-full grid-rows-[auto_1fr_auto]">
      <header
        class="flex h-12 shrink-0 items-center gap-2 border-b border-neutral-800 px-4 shadow-sm"
      >
        <span class="text-lg font-medium text-neutral-500">#</span>
        <span class="font-semibold text-neutral-100">{header_name}</span>
        {#if archived}
          <span
            class="ml-1 rounded bg-neutral-800 px-1.5 py-0.5 text-xs text-neutral-400"
          >
            archived
          </span>
        {/if}

        <!-- Loggability status, top-right. A toggle for viewers with loggability control,
             otherwise a read-only badge. -->
        {#if show_loggability}
          {@const cls = loggable
            ? "bg-amber-600/20 text-amber-400"
            : "bg-neutral-800 text-neutral-500"}
          {#if can_control_loggability}
            <button
              class="ml-auto rounded px-2 py-0.5 text-xs font-medium hover:brightness-125 {cls}"
              title="Toggle whether messages sent here can be logged (autopsied / relayed to bugs)"
              onclick={toggle_loggable}
            >
              Logging {loggable ? "on" : "off"}
            </button>
          {:else}
            <span
              class="ml-auto rounded px-2 py-0.5 text-xs font-medium {cls}"
              title="Whether messages sent here can be logged (autopsied / relayed to bugs)"
            >
              Logging {loggable ? "on" : "off"}
            </span>
          {/if}
        {/if}
      </header>

      <main bind:this={scroller} class="min-h-0 overflow-y-auto py-4">
        {#each events as event, i (event)}
          {#if "Message" in event.data}
            <Message
              sender={display_string(event.data.Message.sender_display)}
              content={event.data.Message.content}
              timestamp={event.timestamp}
              grouped={is_grouped_message(events[i - 1], event)}
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
            <Announcement
              color="#ef4444"
              description="Death"
              content={`${player_name(d.target_id)} has died.\nReal name: ${d.true_name}\nRole: ${d.role}${d.death_message ? `\n\n${d.death_message}` : ""}`}
            />
          {:else if "AnonymousAnnouncement" in event.data}
            <Announcement
              color="#a855f7"
              description="Anonymous Announcement"
              content={event.data.AnonymousAnnouncement.content}
            />
          {:else if "RevealTrueName" in event.data}
            {@const r = event.data.RevealTrueName}
            <Announcement
              color="#3b82f6"
              description="Name Reveal"
              content={`${player_name(r.target_id)}'s true name is ${r.true_name}.`}
            />
          {:else if "RevealNotebookHolding" in event.data}
            {@const r = event.data.RevealNotebookHolding}
            <Announcement
              color="#3b82f6"
              description="Notebook Check"
              content={`${player_name(r.target_id)} is ${r.holding ? "" : "not "}currently holding a notebook.`}
            />
          {:else if "Bugged" in event.data}
            {@const b = event.data.Bugged}
            <Announcement
              color="#eab308"
              description="Surveillance"
              content={b.context === "Custody"
                ? "You are bugged: your messages are being monitored while you are in custody."
                : "You have been bugged. Your messages are being monitored."}
            />
          {:else if "RoleUpdate" in event.data}
            <Announcement
              color="#8b5cf6"
              description="Role"
              content={`Your role is now ${event.data.RoleUpdate.role}.`}
            />
          {:else if "TrueNameUpdate" in event.data}
            <Announcement
              color="#8b5cf6"
              description="True Name"
              content={`Your true name is now ${event.data.TrueNameUpdate.true_name}.`}
            />
          {:else if "PollNotice" in event.data}
            {@const pn = event.data.PollNotice}
            <Announcement
              color="#6366f1"
              description={pn.outcome ? `Vote ${pn.outcome}` : "Vote started"}
              content={poll_notice_text(pn.subject)}
            />
          {:else if "PseudocideRevival" in event.data}
            {@const r = event.data.PseudocideRevival}
            <Announcement
              color="#10b981"
              description="Revival"
              content={`${player_name(r.target_id)} is alive.`}
            />
          {:else if "KidnapReveal" in event.data}
            {@const kr = event.data.KidnapReveal}
            <Announcement
              color="#f59e0b"
              description="Kidnap Reveal"
              content={kr.kidnapper
                ? `${player_name(kr.kidnapper)} was revealed as a kidnapper.`
                : "A kidnapping was revealed, but the kidnapper stayed anonymous."}
            />
          {:else if "Kidnapping" in event.data}
            {@const k = event.data.Kidnapping}
            <!-- TODO: the engine doesn't reveal the victim, so naming them here is
                 wrong and this event is largely useless as-is. Placeholder render
                 until kidnapping either reveals the victim or is reworked/removed. -->
            <Announcement
              color="#f59e0b"
              description="Kidnapping"
              content={`${player_name(k.target_id)} has been kidnapped.`}
            />
          {:else if "ProsecutionEvent" in event.data}
            {@const pe = event.data.ProsecutionEvent}
            <Announcement
              color="#e11d48"
              description={pe.ended ? "Prosecution Ended" : "Prosecution"}
              content={prosecution_event_text(pe)}
            />
          {/if}
        {/each}

        {#if is_news}
          {#if !can_read}
            <!-- news always renders world events (announcements) above; only the
                 chat messages are gated. Spell out the events-vs-messages split. -->
            <div class="px-4 py-3 text-center text-xs text-neutral-500">
              You don't have access to this channel. Announcements above are game
              events and are always shown here — but you can't see chat messages.
            </div>
          {/if}
        {:else if !can_read}
          {#if current_perms?.had_positive}
            <!-- was once a real member but read is now off: show history up to
                 the cutoff, no new messages. -->
            <div class="px-4 py-3 text-center text-xs text-neutral-500">
              You do not have read access to this channel. You will not receive
              new messages — only those up to your access cutoff are shown.
            </div>
          {:else}
            <!-- never a member: no perms entry, or an entry that was never
                 positive (e.g. L & Watari). -->
            <div class="px-4 py-3 text-center text-xs text-neutral-500">
              You don't have access to this channel.
            </div>
          {/if}
        {/if}
        {#if archived}
          <div class="px-4 py-3 text-center text-xs text-neutral-500">
            This channel has been archived.
          </div>
        {/if}
      </main>

      <footer class="shrink-0 px-4 pb-6 pt-1">
        <div class="flex items-center gap-2">
          {#if can_send && available_displays.length > 1}
            <!-- "send as" picker: only shown when the viewer has more than one display -->
            <select
              bind:value={selected_display_key}
              class="rounded-lg bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
            >
              {#each available_displays as d (displayKey(d))}
                <option value={displayKey(d)}>{display_string(d)}</option>
              {/each}
            </select>
          {/if}

          <div class="flex-1">
            {#if can_send}
              <form
                onsubmit={async (event) => {
                  event.preventDefault();
                  await send_message();
                }}
              >
                <div
                  class="flex items-center gap-2 rounded-lg bg-neutral-800 px-2 py-1"
                >
                  <Input
                    bind:value={message_content}
                    placeholder={`Message #${channel_name ?? ""}`}
                    class="flex-1 border-0 bg-transparent shadow-none focus-visible:ring-0 dark:bg-transparent"
                  />
                  <Button
                    size="sm"
                    onclick={async () => {
                      await send_message();
                    }}>Send</Button
                  >
                </div>
              </form>
            {:else if is_bug}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                {archived
                  ? "This surveillance feed is no longer active."
                  : "Read-only surveillance feed."}
              </div>
            {:else if archived}
              <div
                class="rounded-lg bg-neutral-800/50 px-4 py-2.5 text-center text-sm italic text-neutral-500"
              >
                This channel is archived and read-only.
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
            <Button
              size="sm"
              class="bg-red-600 text-white hover:bg-red-700"
              onclick={() => (write_open = true)}>Write</Button
            >
          {/if}
        </div>
      </footer>

      {#if notebook_id}
        <NotebookWrite bind:open={write_open} notebookId={notebook_id} />
      {/if}
    </div>
  {:else}
    <div class="flex h-full items-center justify-center text-neutral-500">
      Select a channel
    </div>
  {/if}
</div>
