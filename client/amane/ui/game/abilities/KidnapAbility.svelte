<script lang="ts">
  // One component for both PublicKidnap and AnonymousKidnap — they differ only in the behaviour
  // key and copy, so the variant is resolved from the ability's own name (looked up the same way
  // AbilityMenu picks its source) rather than duplicated into two files.
  //
  // Public kidnap has one extra wrinkle: an ORG designates which of its own is shown as the
  // kidnapper (a performer picker), while a player is always themselves — so the engine forbids a
  // player from setting a performer. The picker therefore only appears for a public org ability.
  import { getContext } from "svelte";
  import { GAME_STATE_KEY } from "../../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../../client.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { GameState } from "../../../game_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { slotKeyFromString } from "../../../bindings";
  import type { AbilityBehaviour } from "../../../bindings";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import PlayerSelect from "./PlayerSelect.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const game = getContext<GameState>(GAME_STATE_KEY);

  const client = getContext<ClientState>(CLIENT_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  // The set this instance lives in (an org's shared set, or the viewer's own), then its name —
  // PublicKidnap vs AnonymousKidnap decides the copy and the behaviour sent.
  const source = $derived(
    orgId ? game.orgs.get(orgId)?.abilities : game.views.get(ui.viewer)?.abilities,
  );
  const isPublic = $derived(source?.get(abilityId)?.name === "PublicKidnap");
  // Only a public org kidnap gets to choose its public face; a player is always themselves.
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
      // Player: performer must be null (engine enforces). Org: send the chosen face, or null to
      // let the engine default to the acting member.
      const perf = choosesPerformer && performer ? slotKeyFromString(performer) : null;
      behaviour = { PublicKidnap: { target: t, performer: perf } };
    } else {
      behaviour = { AnonymousKidnap: { target: t } };
    }
    const err = await client.dispatch(
      useAbilityRequest(ui.viewer, abilityId, orgId, behaviour),
    );
    if (err) flash.set_error(err);
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
    <!-- Only org members may be the public face (the engine also requires they be present). -->
    <PlayerSelect
      bind:value={performer}
      placeholder="Public face (optional)"
      ids={game.orgs.get(orgId ?? "")?.members ?? []}
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
