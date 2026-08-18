<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import { orgColorVar, orgDisplayName, t } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";
  import Chip from "../Chip.svelte";

  let {
    data,
    view,
    timestamp,
  }: {
    data: {
      target_id: string;
      orgs: { id: string; leader: boolean; og: boolean }[];
    };
    view: GameView;
    timestamp: number;
  } = $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-org-reveal)"
  description="Affiliations"
>
  <Name id={data.target_id} {view} chip /> stood with
  {#each data.orgs as org, i (org.id)}
    {@const name = view.orgs.get(org.id)?.name}
    <Chip
      label={name ? orgDisplayName(name) : t("display_org_unknown")}
      colorVar={name ? orgColorVar(name) : "var(--color-event-reveal)"}
    />{#if org.leader}<span class="text-neutral-400">
        (leader)</span
      >{:else if org.og}<span class="text-neutral-400">
        (OG)</span
      >{/if}{#if i < data.orgs.length - 1},
    {/if}
  {/each}.
</Announcement>
