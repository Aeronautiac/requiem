<script lang="ts">
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "./game_state.svelte.ts";
  import { UI_STATE_KEY } from "./ui_state.svelte.ts";
  import type { GameState, ChannelEvent } from "./game_state.svelte.ts";
  import type { UiState } from "./ui_state.svelte.ts";
  import { ROUTER_KEY } from "$lib/router";
  import type { Router } from "$lib/router";
  import type { ActionRequest, ActorDisplay, ActorKey } from "./bindings";
  import { slotKeyFromString, slotKeyToString } from "./bindings";
  import { viewerToActor } from "./types";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  const router = getContext<Router>(ROUTER_KEY);

  const channel = $derived(
    ui.selected_channel
      ? game.views.get(ui.viewer)?.channels.get(ui.selected_channel)
      : undefined,
  );

  const channel_events = $derived(
    ui.selected_channel ? (game.events.get(ui.selected_channel) ?? []) : [],
  );

  const notebook_id = $derived(
    ui.selected_channel
      ? game.channel_to_notebook.get(ui.selected_channel)
      : undefined,
  );

  let message_content = $state("");
  let write_name = $state("");
  let death_message = $state("");
  let write_delay = $state(0);

  function sender_display(): ActorDisplay {
    if (ui.viewer === "Admin") return "System";
    return { Raw: slotKeyFromString(ui.viewer) };
  }

  function render_display(display: ActorDisplay): string {
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

  function render_writer(user_id: ActorKey): string {
    return (
      game.players.get(slotKeyToString(user_id))?.display_name ?? "Unknown"
    );
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
  }

  async function submit_write_name() {
    if (!notebook_id || !write_name.trim()) return;
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: Date.now(),
      payload: {
        WriteName: {
          notebook_id,
          true_name: write_name.trim(),
          death_message: death_message.trim() || null,
          delay: write_delay,
        },
      },
    };
    game.process_response(await router.sendAction(request));
    write_name = "";
    death_message = "";
    write_delay = 0;
  }
</script>

{#if !ui.selected_channel || !channel}
  <div class="flex items-center justify-center h-full text-neutral-600 text-sm">
    Select a channel
  </div>
{:else}
  <div class="flex flex-col h-full">
    <div
      class="px-4 py-2 border-b border-neutral-800 text-sm font-medium text-neutral-300"
    >
      {channel.name}
    </div>

    <div class="flex-1 overflow-y-auto px-4 py-2 flex flex-col gap-2">
      {#if channel.archived}
        <p class="text-xs text-neutral-500 text-center mt-2">
          This channel has been archived and can no longer be interacted with.
        </p>
      {:else if !channel.read}
        <p class="text-xs text-neutral-500 text-center mt-2">
          You are not receiving new messages. You do not possess view
          permissions.
        </p>
      {/if}

      {#each channel_events as event (event.timestamp)}
        {#if event.type === "message"}
          <div class="text-sm">
            <span class="font-medium text-neutral-300"
              >{render_display(event.sender_display)}</span
            >
            <span class="text-neutral-500 text-xs ml-1"
              >{new Date(event.timestamp).toLocaleTimeString()}</span
            >
            <p class="text-neutral-400">{event.content}</p>
          </div>
        {:else if event.type === "notebook_write"}
          <div
            class="text-xs rounded bg-neutral-900 px-3 py-2 border-l-2 {event.success
              ? 'border-red-700'
              : 'border-neutral-700'}"
          >
            <span
              class="font-medium {event.success
                ? 'text-red-400'
                : 'text-neutral-400'}">{render_writer(event.user_id)}</span
            >
            <span class="text-neutral-500 ml-1"
              >{new Date(event.timestamp).toLocaleTimeString()}</span
            >
            <p class="mt-0.5 text-neutral-300">
              <span>Wrote "{event.true_name}</span>
              {#if event.success}
                <span class="text-red-400"
                  >{event.target_saved
                    ? " — Something saved them..."
                    : event.delay > 0
                      ? `— dies in ${event.delay}s"`
                      : ""}</span
                >
              {:else}
                <span class="text-neutral-500">no match</span>
              {/if}
            </p>
            {#if event.message}
              <p class="text-neutral-500 italic mt-0.5">"{event.message}"</p>
            {/if}
            <p class="text-neutral-600 mt-0.5">
              {#if event.attempts_remaining > 0}
                {event.successes_remaining} writes remaining
              {/if}
              {#if event.successes_remaining > 0 && event.attempts_remaining > 0}
                ·
              {/if}
              {#if event.successes_remaining > 0}
                {event.attempts_remaining} attempts remaining
              {/if}
            </p>
          </div>
        {/if}
      {/each}
    </div>

    {#if notebook_id && ui.viewer !== "Admin" && !channel.archived}
      <div class="px-4 py-2 border-t border-neutral-800 flex flex-col gap-1.5">
        <p class="text-xs text-neutral-500 font-medium">Write a name</p>
        <input
          bind:value={write_name}
          placeholder="True name..."
          class="bg-neutral-900 rounded px-3 py-1.5 text-sm text-neutral-200 outline-none focus:ring-1 focus:ring-red-800"
        />
        <div class="flex gap-2">
          <input
            bind:value={death_message}
            placeholder="Death message (optional)..."
            class="flex-1 bg-neutral-900 rounded px-3 py-1.5 text-sm text-neutral-200 outline-none focus:ring-1 focus:ring-neutral-600"
          />
          <input
            type="number"
            bind:value={write_delay}
            min="0"
            placeholder="Delay"
            class="w-20 bg-neutral-900 rounded px-3 py-1.5 text-sm text-neutral-200 outline-none focus:ring-1 focus:ring-neutral-600"
          />
          <button
            onclick={submit_write_name}
            class="px-3 py-1.5 rounded bg-red-900 hover:bg-red-800 text-sm text-white"
          >
            Write
          </button>
        </div>
      </div>
    {/if}

    <div class="px-4 py-2 border-t border-neutral-800">
      {#if channel.archived}
        <div class="px-3 py-2 rounded bg-neutral-900 text-neutral-600 text-sm">
          Channel archived — cannot send messages.
        </div>
      {:else if !channel.send || ui.viewer === "Admin"}
        <div class="px-3 py-2 rounded bg-neutral-900 text-neutral-600 text-sm">
          {ui.viewer === "Admin"
            ? "Observing — send disabled for admin."
            : "You don't have permission to send messages."}
        </div>
      {:else}
        <div class="flex gap-2">
          <input
            bind:value={message_content}
            onkeydown={(e) => e.key === "Enter" && send_message()}
            placeholder="Send a message..."
            class="flex-1 bg-neutral-900 rounded px-3 py-2 text-sm text-neutral-200 outline-none focus:ring-1 focus:ring-neutral-600"
          />
          <button
            onclick={send_message}
            class="px-3 py-2 rounded bg-neutral-700 hover:bg-neutral-600 text-sm text-white"
          >
            Send
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
