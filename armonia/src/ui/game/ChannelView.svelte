<script lang="ts">
  import { getContext } from "svelte";
  import Input from "$lib/components/ui/input/input.svelte";
  import { GAME_STATE_KEY } from "../../game_state.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import type { ChannelEvent, GameState } from "../../game_state.svelte.ts";
  import type { UiState } from "../../ui_state.svelte.ts";
  import { ROUTER_KEY } from "$lib/router";
  import type { Router } from "$lib/router";
  import type { ActionRequest, ActorDisplay } from "../../bindings";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import Button from "$lib/components/ui/button/button.svelte";
  import Message from "./Message.svelte";
  import NotebookWrite from "./NotebookWrite.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const router = getContext<Router>(ROUTER_KEY);

  let message_content = $state("");
  let channel_name = $derived(get_channel_name());
  let scroller = $state<HTMLElement>();

  // Admin implicitly has every permission; players go by their channel_perms entry
  // (which may be absent, meaning no access).
  const is_admin = $derived(ui.viewer === "Admin");
  const current_channel = $derived(
    ui.selected_channel ? game.channels.get(ui.selected_channel) : undefined,
  );
  const current_perms = $derived(
    ui.selected_channel
      ? game.views.get(ui.viewer)?.channel_perms.get(ui.selected_channel)
      : undefined,
  );
  const archived = $derived(current_channel?.archived ?? false);
  // Archived channels are read-only for everyone, admin included.
  const can_send = $derived(
    !archived && (is_admin || (current_perms?.send ?? false)),
  );
  const can_read = $derived(is_admin || (current_perms?.read ?? false));
  const is_notebook = $derived(current_channel?.kind === "Notebook");
  const notebook_id = $derived(
    ui.selected_channel
      ? game.notebook_for_channel(ui.selected_channel)
      : undefined,
  );
  let write_open = $state(false);

  function get_channel_name(): string | null {
    return game.channels.get(ui.selected_channel ?? "")?.name ?? null;
  }

  function sender_display(): ActorDisplay {
    if (ui.viewer === "Admin") return "System";
    return { Raw: slotKeyFromString(ui.viewer) };
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

  async function send_message() {
    if (!ui.selected_channel || !message_content.trim()) return;
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: Date.now(),
      payload: {
        SendMessage: {
          channel_id: slotKeyFromString(ui.selected_channel),
          display: sender_display(),
          content: message_content.trim(),
        },
      },
    };
    game.process_response(await router.sendAction(request));
    message_content = "";
    console.log("message sent");
  }

  // based on the context, choose which events to include, push them into a unified list, and then sort the list
  const events: ChannelEvent[] = $derived.by(() => {
    if (!ui.selected_channel) {
      return [];
    }

    // The selected channel can outlive access to it: switching to a viewer with
    // no perms entry, or a channel being removed while selected. Both leave us
    // without a channel/perms to read, so bail out with an empty list.
    const channel = current_channel;
    if (!channel) {
      return [];
    }

    let accum: ChannelEvent[] = [];

    // Full history if admin or currently readable; if read was revoked but once
    // existed, only messages up to the cutoff; no perms entry at all → nothing.
    if (can_read) {
      accum = accum.concat(channel.events);
    } else if (current_perms) {
      accum = accum.concat(
        channel.events.filter(
          (val) => val.timestamp <= current_perms.read_updated,
        ),
      );
    }

    // TODO:
    // world events and similar

    // oldest to newest
    accum.sort((a, b) => a.timestamp - b.timestamp);

    return accum;
  });

  // Switching channels always lands at the bottom. The <main> element is reused
  // across channels, so its scrollTop would otherwise carry over from the last one.
  $effect(() => {
    ui.selected_channel; // track: re-run on channel switch
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
  {#if ui.selected_channel}
    <div class="grid h-full w-full grid-rows-[auto_1fr_auto]">
      <header
        class="flex h-12 shrink-0 items-center gap-2 border-b border-neutral-800 px-4 shadow-sm"
      >
        <span class="text-lg font-medium text-neutral-500">#</span>
        <span class="font-semibold text-neutral-100">{channel_name}</span>
        {#if archived}
          <span
            class="ml-1 rounded bg-neutral-800 px-1.5 py-0.5 text-xs text-neutral-400"
          >
            archived
          </span>
        {/if}
      </header>

      <main bind:this={scroller} class="min-h-0 overflow-y-auto py-4">
        {#each events as event (event)}
          {#if "Message" in event.data}
            <Message
              sender={display_string(event.data.Message.sender_display)}
              content={event.data.Message.content}
              timestamp={event.timestamp}
            />
          {:else if "World" in event.data}
            <div class="px-4 py-1 text-xs italic text-neutral-500">
              World event
            </div>
          {:else if "Write" in event.data}
            <div class="px-4 py-1 text-xs italic text-neutral-500">
              Notebook write
            </div>
          {/if}
        {/each}

        {#if !can_read}
          {#if current_perms}
            <!-- read is off but a perms entry exists (revoked, or spawned without
                 read): show history up to the cutoff, no new messages. -->
            <div class="px-4 py-3 text-center text-xs text-neutral-500">
              You do not have read access to this channel. You will not receive
              new messages — only those up to your access cutoff are shown.
            </div>
          {:else}
            <!-- no perms entry at all (e.g. switched to a view without access) -->
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

          {#if is_notebook && notebook_id}
            <Button
              size="sm"
              class="bg-red-600 text-white hover:bg-red-700"
              onclick={() => (write_open = true)}>Write</Button
            >
          {/if}
        </div>

        <!-- TODO: toggle loggability -->
      </footer>

      {#if is_notebook && notebook_id}
        <NotebookWrite bind:open={write_open} notebookId={notebook_id} />
      {/if}
    </div>
  {:else}
    <div class="flex h-full items-center justify-center text-neutral-500">
      Select a channel
    </div>
  {/if}
</div>
