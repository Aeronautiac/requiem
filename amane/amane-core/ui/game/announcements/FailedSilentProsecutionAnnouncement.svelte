<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { OrganizationName } from "../../../bindings";
  import { nameLabel, orgDisplayName } from "../../../game/helpers.svelte";
  import Announcement from "../Announcement.svelte";
  import Name from "../Name.svelte";

  let {
    data,
    view,
    timestamp,
  }: {
    data: { accuser_id: string; true_name: string; org: OrganizationName };
    view: GameView;
    timestamp: number;
  } = $props();
</script>

<Announcement
  {view}
  {timestamp}
  color="var(--color-event-prosecution)"
  description="Failed Silent Prosecution"
>
  <Name id={data.accuser_id} {view} chip /> has attempted to execute an innocent
  person of HEINOUS crimes...
  <div class="mt-1">
    True name: <span class="text-neutral-200">{nameLabel(data.true_name)}</span>
  </div>
  <div class="mt-1">
    {orgDisplayName(data.org)} will NOT accept this, and so, they've been permanently
    banned from the organization.
  </div>
</Announcement>
