<script lang="ts">
  import { execErrorText } from "../../../game/helpers.svelte";
  import { getContext } from "svelte";
  import { SESSION_KEY, type SessionState } from "../../../session.svelte.ts";
  import { UI_STATE_KEY } from "../../../ui_state.svelte.ts";
  import type { UiState } from "../../../ui_state.svelte.ts";
  import { Flash } from "../../../flash.svelte.ts";
  import FlashDisplay from "../../Flash.svelte";
  import { useAbilityRequest, type AbilityUiProps } from "./registry";

  let { abilityId, onDone, orgId }: AbilityUiProps = $props();

  const session = getContext<SessionState>(SESSION_KEY);
  const ui = getContext<UiState>(UI_STATE_KEY);

  const flash = new Flash();

  async function go_dark() {
    const reply = await session.submit_action(
      useAbilityRequest(ui.viewer, abilityId, orgId, { Blackout: {} }),
    );
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else onDone();
  }
</script>

<div class="flex flex-col gap-3">
  <p class="text-sm text-ink-dim">
    Take the world dark. Nothing that happens is announced to anyone while it lasts, and the
    news goes off the air. Nothing is lost — everything held back arrives at once when it
    lifts.
  </p>
  <p class="text-sm text-ink-dim">
    Players are still told who joins and what day it is, so an absence can be worked out from
    a roster. What they cannot learn is what became of anybody.
  </p>
  <!-- Stated plainly: the mark is the entire price, and it is the thing a voter needs to weigh. -->
  <p class="text-sm text-danger">
    Everyone in the organization right now is permanently marked as wanted, and leaving later
    does not undo it. The organization itself is marked too, which exposes anyone who joins
    afterwards for as long as they stay.
  </p>
  <button
    class="rounded-md bg-danger px-3 py-2 text-sm font-medium text-danger-ink hover:brightness-110"
    onclick={go_dark}
  >
    Cut the lights
  </button>
  <FlashDisplay {flash} />
</div>
