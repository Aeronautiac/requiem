<script lang="ts">
  import { execErrorText, orgDisplayName, roleLabel } from "../../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game/state.svelte";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import type { ActorKey, OrgMemberView, Role } from "../../../bindings";
  import { slotKeyFromString } from "../../../bindings";
  import { ROLES } from "../../../constants";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import MentionInput from "../MentionInput.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const view = $derived(game.view_of(ui.viewer));

  let target = $state("");
  let true_name = $state("");
  let death_message = $state("");
  let role = $state<Role>(ROLES[0]);
  let notebook_transferred = $state(false);
  let ability_transferred = $state(false);
  // Fabricated affiliations to show on the fake death. Every org rides the data viewport, so any of
  // them can be named; each entry carries whatever leader/OG standing the faker wants seen. Keyed by
  // org actor key; absence means that org is not part of the lie.
  let org_reveal = $state<Record<string, { leader: boolean; og: boolean }>>({});
  const flash = new Flash();

  function toggle_org(key: string) {
    if (org_reveal[key]) delete org_reveal[key];
    else org_reveal[key] = { leader: false, og: false };
  }

  async function fake_death() {
    if (!target) {
      flash.set_error("Pick whose death to fake.");
      return;
    }
    if (!true_name.trim()) {
      flash.set_error("A true name is required.");
      return;
    }
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, {
        Pseudocide: {
          target_id: slotKeyFromString(target),
          true_name,
          // Blank means "use the default death message" — send None, not an empty string.
          death_message: death_message.trim() ? death_message : null,
          role,
          orgs: Object.entries(org_reveal).map(
            ([key, v]) =>
              [slotKeyFromString(key), { leader: v.leader, og: v.og }] as [ActorKey, OrgMemberView],
          ),
          notebook_transferred,
          ability_transferred,
        },
      }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">

  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    Target
    <PlayerSelect bind:value={target} placeholder="Whose death to fake" />
  </label>

  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    True name revealed
    <input
      bind:value={true_name}
      placeholder="True name"
      class="w-full rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
    />
  </label>

  <div class="flex flex-col gap-1 text-xs text-neutral-500">
    <span class="w-full">Death message</span>
    <MentionInput
      bind:value={death_message}
      players={view.players}
      orgs={view.orgs}
      newsAnchor={view.news_anchor}
      pressConf={view.press_conf}
      statuses={view.actor_statuses}
      placeholder="Announced on death"
      boxed
      onsubmit={fake_death}
    />
  </div>

  <label class="flex flex-col gap-1 text-xs text-neutral-500">
    Role revealed
    <select
      bind:value={role}
      class="w-full rounded-md bg-neutral-800 px-2 py-2 text-sm text-neutral-200"
    >
      {#each ROLES as r (r)}
        <option value={r}>{roleLabel(r)}</option>
      {/each}
    </select>
  </label>

  <div class="flex flex-col gap-1 text-xs text-neutral-500">
    Affiliations revealed
    <div class="flex flex-col gap-1 rounded-md bg-neutral-800 p-2">
      {#each [...view.orgs] as [key, org] (key)}
        <div class="flex items-center gap-2">
          <label class="flex flex-1 items-center gap-2 text-sm text-neutral-300">
            <input
              type="checkbox"
              checked={!!org_reveal[key]}
              onchange={() => toggle_org(key)}
            />
            {orgDisplayName(org.name)}
          </label>
          {#if org_reveal[key]}
            <label class="flex items-center gap-1 text-xs text-neutral-400">
              <input type="checkbox" bind:checked={org_reveal[key].leader} /> leader
            </label>
            <label class="flex items-center gap-1 text-xs text-neutral-400">
              <input type="checkbox" bind:checked={org_reveal[key].og} /> og
            </label>
          {/if}
        </div>
      {:else}
        <span class="text-neutral-500">No organizations to name.</span>
      {/each}
    </div>
  </div>

  <label class="flex items-center gap-2 text-sm text-neutral-300">
    <input type="checkbox" bind:checked={notebook_transferred} />
    Notebook transferred
  </label>
  <label class="flex items-center gap-2 text-sm text-neutral-300">
    <input type="checkbox" bind:checked={ability_transferred} />
    Abilities transferred
  </label>

  <button
    class="rounded-md bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-900 hover:bg-white"
    onclick={fake_death}
  >
    Fake death
  </button>
  <FlashDisplay {flash} />
</div>
