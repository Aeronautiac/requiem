<script lang="ts">
  import type { GameView } from "../../../game/view.svelte";
  import type { ActorDisplay, ProsecutionPhaseView } from "../../../bindings";
  import Announcement from "../Announcement.svelte";
  import ActorDisplayChip from "../ActorDisplay.svelte";

  let { data, view, timestamp }: {
    data: {
      prosecutor_display: ActorDisplay;
      defendant_display: ActorDisplay;
      phase: ProsecutionPhaseView;
      ended: boolean;
      verdict: boolean | null;
    };
    view: GameView;
    timestamp: number;
  } = $props();
</script>

{#snippet defendant()}<ActorDisplayChip display={data.defendant_display} {view} />{/snippet}
{#snippet prosecutor()}<ActorDisplayChip display={data.prosecutor_display} {view} />{/snippet}

<Announcement {view} {timestamp}
  color="var(--color-event-prosecution)"
  description={data.ended ? "Prosecution Ended" : "Prosecution"}
>
  {#if data.ended}
    {#if data.verdict === true}
      {@render defendant()} has been found guilty.
    {:else if data.verdict === false}
      {@render defendant()} has been acquitted.
    {:else}
      The prosecution of {@render defendant()} has ended.
    {/if}
  {:else if data.phase === "Voting"}
    The trial vote for {@render defendant()} has begun.
  {:else if "Custody" in data.phase}
    {@render prosecutor()} is prosecuting {@render defendant()}.
  {:else if "Debate" in data.phase.Trial}
    The trial of {@render defendant()} has entered debate.
  {:else if "Prosecutor" in data.phase.Trial}
    {#if data.phase.Trial.Prosecutor === "Grace"}
      The trial of {@render defendant()} has begun, the prosecution has the floor.
    {:else}
      In the trial of {@render defendant()}, the prosecution presents.
    {/if}
  {:else}
    {#if data.phase.Trial.Defense === "Grace"}
      In the trial of {@render defendant()}, the defense has the floor.
    {:else}
      In the trial of {@render defendant()}, the defense presents.
    {/if}
  {/if}
</Announcement>
