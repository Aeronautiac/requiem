<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import { orgColorVar, orgDisplayName, t } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";
  import Chip from "../Chip.svelte";

  let { data, view, timestamp }: { data: { org_id: string; leader: boolean }; view: GameView; timestamp: number } = $props();
  const org_name = view.orgs.get(data.org_id)?.name;
</script>

<Announcement {view} {timestamp} color="var(--color-event-personal)" description="Leadership">
  {#if data.leader}You are now the leader of{:else}You are no longer the leader of{/if}
  <Chip
    label={org_name ? orgDisplayName(org_name) : t("display_org_unknown")}
    colorVar={org_name ? orgColorVar(org_name) : "var(--color-event-personal)"}
  />.
</Announcement>
