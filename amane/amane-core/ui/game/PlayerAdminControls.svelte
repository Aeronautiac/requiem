<script lang="ts">
  // Everything here dispatches as the Admin actor, which the engine accepts for these.
  import { execErrorText, nameLabel, roleLabel } from "../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../ui_state.svelte.ts";
  import { now } from "../../time.svelte.ts";
  import type { GameState } from "../../game/state.svelte";
  import type { UiState } from "../../ui_state.svelte.ts";
  import type { Action, ActionRequest, Role } from "../../bindings";
  import { slotKeyFromString } from "../../bindings";
  import { viewerToActor } from "../../types";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  interface Props {
    id: string;
  }
  let { id }: Props = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));
  const flash = new Flash();

  // Hand-maintained to mirror the Role union in bindings (no runtime enum to derive from).
  const ROLES: Role[] = [
    "Kira", "SecondKira", "L", "Watari", "BeyondBirthday", "PrivateInvestigator",
    "Civilian", "RogueCivilian", "Poser", "ConArtist", "WantedCivilian",
    "Near", "Mello",
  ];

  const info = $derived(view.player_info.get(id));
  const target = $derived(slotKeyFromString(id));

  // For picking a NEW role; the current one is shown in the inspector below, so this just defaults
  // rather than tracking `info`, which would only capture its initial value.
  let role = $state<Role>("Civilian");
  let true_name = $state("");
  let display_name = $state("");

  async function run(payload: Action, ok: string) {
    const request: ActionRequest = {
      actor: viewerToActor(ui.viewer),
      timestamp: now(),
      payload,
    };
    const reply = await session.submit_action(request);
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success(ok);
  }

  function set_role() {
    run({ GiveRole: { target_id: target, role } }, `Role set to ${roleLabel(role)}.`);
  }
  function set_true_name() {
    const name = true_name.trim();
    if (!name) {
      flash.set_error("Enter a name.");
      return;
    }
    run({ SetTrueName: { target_id: target, true_name: name } }, "True name set.");
  }
  // A CONTROL, not an action: a profile is the server's record of who is playing the slot, and the
  // engine has no concept of it. True name above is the opposite — a mechanic, secret, and the
  // thing you write in a notebook.
  //
  // The control replaces the whole profile, so this states every field. As Profile grows this form
  // grows with it rather than sprouting a control per field.
  async function set_profile() {
    const name = display_name.trim();
    if (!name) {
      flash.set_error("Enter a name.");
      return;
    }
    const reply = await session.submit_control({
      Sim: {
        time: 0,
        data: { SetProfile: { actor: target, profile: { display_name: name } } },
      },
    });
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else flash.set_success("Profile updated.");
  }
  // News anchor is a status, not a role — handing over the anchor's kit rather than changing who
  // the player is. The engine rejects naming the current anchor again, surfaced as a flash.
  function make_news_anchor() {
    run({ SetNewsAnchor: { target_id: target } }, "Set as news anchor.");
  }
  // Vacate the post entirely — there is only one anchor, so target_id: null takes it off whoever
  // holds it without handing it on. The engine rejects this when nobody holds it (AlreadyNewsAnchor).
  function clear_news_anchor() {
    run({ SetNewsAnchor: { target_id: null } }, "News anchor post vacated.");
  }
  function kill() {
    run(
      {
        Kill: {
          target_id: target,
          killer_id: null,
          death_message: null,
          silent: false,
          allow_link_chaining: true,
          sever_links: true,
          set_books_dormant: false,
        },
      },
      "Player killed.",
    );
  }
  function revive() {
    run({ Revive: { target_id: target, ignore_links: false } }, "Player revived.");
  }
</script>

<div class="flex flex-col gap-2 py-1 text-sm">
  <div class="text-neutral-500">
    <div><span class="text-neutral-600">Role:</span> {info?.role ? roleLabel(info.role) : "—"}</div>
    <div>
      <span class="text-neutral-600">True name:</span>
      {info?.true_name ? nameLabel(info.true_name) : "—"}
    </div>
  </div>

  <div class="flex items-center gap-1.5">
    <select
      bind:value={role}
      class="min-w-0 flex-1 rounded bg-neutral-800 px-2 py-1.5 text-neutral-200"
    >
      {#each ROLES as r (r)}
        <option value={r}>{roleLabel(r)}</option>
      {/each}
    </select>
    <button
      class="shrink-0 rounded bg-neutral-700 px-3 py-1.5 text-neutral-200 hover:bg-neutral-600"
      onclick={set_role}
    >
      Set role
    </button>
  </div>

  <div class="flex items-center gap-1.5">
    <button
      class="flex-1 rounded bg-neutral-700 px-3 py-1.5 text-neutral-200 hover:bg-neutral-600"
      onclick={make_news_anchor}
    >
      Set as news anchor
    </button>
    <button
      class="shrink-0 rounded bg-neutral-700 px-3 py-1.5 text-neutral-200 hover:bg-neutral-600"
      onclick={clear_news_anchor}
    >
      Vacate
    </button>
  </div>

  <div class="flex items-center gap-1.5">
    <input
      bind:value={true_name}
      placeholder="New true name"
      class="min-w-0 flex-1 rounded bg-neutral-800 px-2 py-1.5 text-neutral-200"
    />
    <button
      class="shrink-0 rounded bg-neutral-700 px-3 py-1.5 text-neutral-200 hover:bg-neutral-600"
      onclick={set_true_name}
    >
      Set true name
    </button>
  </div>

  <div class="flex items-center gap-1.5">
    <input
      bind:value={display_name}
      placeholder="New display name"
      class="min-w-0 flex-1 rounded bg-neutral-800 px-2 py-1.5 text-neutral-200"
    />
    <button
      class="shrink-0 rounded bg-neutral-700 px-3 py-1.5 text-neutral-200 hover:bg-neutral-600"
      onclick={set_profile}
    >
      Set display name
    </button>
  </div>

  <div class="flex items-center gap-1.5">
    <button
      class="flex-1 rounded bg-red-900/60 px-3 py-1.5 text-red-200 hover:bg-red-900"
      onclick={kill}
    >
      Kill
    </button>
    <button
      class="flex-1 rounded bg-emerald-900/60 px-3 py-1.5 text-emerald-200 hover:bg-emerald-900"
      onclick={revive}
    >
      Revive
    </button>
  </div>

  <FlashDisplay {flash} />
</div>
