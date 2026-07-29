<script lang="ts">
  // One component for both PublicKidnap and AnonymousKidnap — they differ only in the behaviour
  // key and copy, so the variant is resolved from the ability's own name.
  //
  // Public kidnap has one wrinkle: an ORG designates which of its own is shown as the kidnapper,
  // while a player is always themselves, and the engine forbids a player from setting a performer.
  // So the picker only appears for a public org ability.
  import { execErrorText } from "../../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game/state.svelte";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import type { AbilityBehaviour } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);
  // The one view this component renders. Everything it can show is what this view was delivered.
  const view = $derived(game.view_of(ui.viewer));

  // The set this instance lives in — an org's shared set, or the viewer's own.
  const source = $derived(
    orgId ? view.orgs.get(orgId)?.abilities : view.abilities,
  );
  const isPublic = $derived(source?.get(abilityId)?.name === "PublicKidnap");
  // Only a public org kidnap chooses its public face; a player is always themselves.
  const choosesPerformer = $derived(isPublic && orgId != null);

  let target = $state("");
  let performer = $state(""); // only used when choosesPerformer
  const flash = new Flash();

  async function run() {
    if (!target) {
      flash.set_error("Pick a target.");
      return;
    }
    const t = slotKeyFromString(target);
    let behaviour: AbilityBehaviour;
    if (isPublic) {
      // A player's performer must be null; an org sends the chosen face, or null to default to
      // the acting member.
      const perf = choosesPerformer && performer ? slotKeyFromString(performer) : null;
      behaviour = { PublicKidnap: { target: t, performer: perf } };
    } else {
      behaviour = { AnonymousKidnap: { target: t } };
    }
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, behaviour),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-neutral-400">
    {#if isPublic}
      Kidnap a player: they're pulled into a private channel until released. When it ends,
      <span class="text-neutral-200">the kidnapper is revealed</span>.
    {:else}
      Kidnap a player: they're pulled into a private channel until released. The kidnapping is
      <span class="text-neutral-200">anonymous</span> — the kidnapper stays hidden on release.
    {/if}
  </p>

  <label class="text-xs text-neutral-500">Target</label>
  <PlayerSelect bind:value={target} placeholder="Who to kidnap" />

  {#if choosesPerformer}
    <label class="text-xs text-neutral-500">
      Shown as the kidnapper (defaults to whoever acts)
    </label>
    <!-- The engine also requires they be present. -->
    <PlayerSelect
      bind:value={performer}
      placeholder="Public face (optional)"
      ids={view.orgs.get(orgId ?? "")?.members ?? []}
    />
  {/if}

  <button
    class="rounded-md bg-amber-600 px-3 py-2 text-sm font-medium text-white hover:bg-amber-500"
    onclick={run}
  >
    Kidnap
  </button>
  <FlashDisplay {flash} />
</div>
