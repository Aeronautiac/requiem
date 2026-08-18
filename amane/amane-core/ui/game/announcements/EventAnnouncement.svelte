<script lang="ts">
  // Dispatches a single non-message event to its own rendering handler. Each handler knows how to
  // draw its one event type as an Announcement (or its dedicated card), so ChannelView stays a
  // short dispatch rather than a long inline chain. Each branch narrows the event and hands the
  // handler exactly the data it renders.
  import type { GameView } from "../../../game/view.svelte";
  import type { GameEvent } from "../../../game/types";
  import type { PollData, PollView } from "../../../game/types";
  import WriteAnnouncement from "./WriteAnnouncement.svelte";
  import DeathAnnouncement from "./DeathAnnouncement.svelte";
  import DeathRoleAnnouncement from "./DeathRoleAnnouncement.svelte";
  import DeathOrgsAnnouncement from "./DeathOrgsAnnouncement.svelte";
  import DeathTransferAnnouncement from "./DeathTransferAnnouncement.svelte";
  import AnonymousAnnouncement from "./AnonymousAnnouncement.svelte";
  import EyeDealAnnouncement from "./EyeDealAnnouncement.svelte";
  import NewsAnchorAnnouncement from "./NewsAnchorAnnouncement.svelte";
  import NewsAnchorStatusAnnouncement from "./NewsAnchorStatusAnnouncement.svelte";
  import PressConfMembershipAnnouncement from "./PressConfMembershipAnnouncement.svelte";
  import LeaderStatusAnnouncement from "./LeaderStatusAnnouncement.svelte";
  import PressConfStatusAnnouncement from "./PressConfStatusAnnouncement.svelte";
  import FailedSilentProsecutionAnnouncement from "./FailedSilentProsecutionAnnouncement.svelte";
  import RevealTrueNameAnnouncement from "./RevealTrueNameAnnouncement.svelte";
  import RevealNotebookHoldingAnnouncement from "./RevealNotebookHoldingAnnouncement.svelte";
  import EyeCountAnnouncement from "./EyeCountAnnouncement.svelte";
  import BuggedAnnouncement from "./BuggedAnnouncement.svelte";
  import RoleUpdateAnnouncement from "./RoleUpdateAnnouncement.svelte";
  import TrueNameUpdateAnnouncement from "./TrueNameUpdateAnnouncement.svelte";
  import NotebookReceivedAnnouncement from "./NotebookReceivedAnnouncement.svelte";
  import PollCard from "../PollCard.svelte";
  import PollNoticeCard from "../PollNoticeCard.svelte";
  import PseudocideRevivalAnnouncement from "./PseudocideRevivalAnnouncement.svelte";
  import KidnapRevealAnnouncement from "./KidnapRevealAnnouncement.svelte";
  import KidnappingAnnouncement from "./KidnappingAnnouncement.svelte";
  import IncarcerationAnnouncement from "./IncarcerationAnnouncement.svelte";
  import IncarcerationReleasedAnnouncement from "./IncarcerationReleasedAnnouncement.svelte";
  import NewIterationAnnouncement from "./NewIterationAnnouncement.svelte";
  import BlackoutAnnouncement from "./BlackoutAnnouncement.svelte";
  import ChannelTappedAnnouncement from "./ChannelTappedAnnouncement.svelte";
  import TapInResultAnnouncement from "./TapInResultAnnouncement.svelte";
  import FakeLoungeTappedAnnouncement from "./FakeLoungeTappedAnnouncement.svelte";
  import KiraConnectionAttemptAnnouncement from "./KiraConnectionAttemptAnnouncement.svelte";
  import ContactLogRow from "../ContactLogRow.svelte";
  import ProsecutionAnnouncement from "./ProsecutionAnnouncement.svelte";

  let {
    event,
    view,
    timestamp,
  }: { event: GameEvent; view: GameView; timestamp: number } = $props();

  // A poll's start notice rides its home channel's stream. While the poll is still live we render
  // the interactive card in place of the "vote started" announcement; once it resolves, the
  // announcement (with its outcome) takes over again. Absent/resolved polls answer null and fall
  // through to the announcement.
  function live_poll(
    poll_id: string,
  ): { data: PollData; pollView: PollView | null; frozen: boolean } | null {
    const data = view.polls.get(poll_id);
    if (!data || data.outcome) return null;
    return {
      data,
      pollView: view.poll_views.get(poll_id) ?? null,
      frozen: view.frozen(view.poll_viewport(poll_id)),
    };
  }
</script>

{#if "Write" in event.data}
  <WriteAnnouncement data={event.data.Write} {view} {timestamp} />
{:else if "Death" in event.data}
  <DeathAnnouncement data={event.data.Death} {view} {timestamp} />
{:else if "DeathRole" in event.data}
  <DeathRoleAnnouncement data={event.data.DeathRole} {view} {timestamp} />
{:else if "DeathOrgs" in event.data}
  <DeathOrgsAnnouncement data={event.data.DeathOrgs} {view} {timestamp} />
{:else if "DeathTransfer" in event.data}
  <DeathTransferAnnouncement
    data={event.data.DeathTransfer}
    {view}
    {timestamp}
  />
{:else if "AnonymousAnnouncement" in event.data}
  <AnonymousAnnouncement
    data={event.data.AnonymousAnnouncement}
    {view}
    {timestamp}
  />
{:else if "EyeDealTaken" in event.data}
  <EyeDealAnnouncement data={event.data.EyeDealTaken} {view} {timestamp} />
{:else if "NewsAnchor" in event.data}
  <NewsAnchorAnnouncement data={event.data.NewsAnchor} {view} {timestamp} />
{:else if "NewsAnchorStatus" in event.data}
  <NewsAnchorStatusAnnouncement
    data={event.data.NewsAnchorStatus}
    {view}
    {timestamp}
  />
{:else if "PressConfMembership" in event.data}
  <PressConfMembershipAnnouncement
    data={event.data.PressConfMembership}
    {view}
    {timestamp}
  />
{:else if "LeaderStatus" in event.data}
  <LeaderStatusAnnouncement data={event.data.LeaderStatus} {view} {timestamp} />
{:else if "PressConfStatus" in event.data}
  <PressConfStatusAnnouncement
    data={event.data.PressConfStatus}
    {view}
    {timestamp}
  />
{:else if "FailedSilentProsecution" in event.data}
  <FailedSilentProsecutionAnnouncement
    data={event.data.FailedSilentProsecution}
    {view}
    {timestamp}
  />
{:else if "RevealTrueName" in event.data}
  <RevealTrueNameAnnouncement
    data={event.data.RevealTrueName}
    {view}
    {timestamp}
  />
{:else if "RevealNotebookHolding" in event.data}
  <RevealNotebookHoldingAnnouncement
    data={event.data.RevealNotebookHolding}
    {view}
    {timestamp}
  />
{:else if "EyeCount" in event.data}
  <EyeCountAnnouncement data={event.data.EyeCount} {view} {timestamp} />
{:else if "Bugged" in event.data}
  <BuggedAnnouncement data={event.data.Bugged} {view} {timestamp} />
{:else if "RoleUpdate" in event.data}
  <RoleUpdateAnnouncement data={event.data.RoleUpdate} {view} {timestamp} />
{:else if "TrueNameUpdate" in event.data}
  <TrueNameUpdateAnnouncement
    data={event.data.TrueNameUpdate}
    {view}
    {timestamp}
  />
{:else if "NotebookReceived" in event.data}
  <NotebookReceivedAnnouncement
    data={event.data.NotebookReceived}
    {view}
    {timestamp}
  />
{:else if "PollNotice" in event.data}
  {@const pn = event.data.PollNotice}
  {@const live = pn.outcome ? null : live_poll(pn.poll_id)}
  {#if live}
    <div class="px-3 py-1" data-poll-anchor={pn.poll_id}>
      <PollCard
        id={pn.poll_id}
        data={live.data}
        pollView={live.pollView}
        frozen={live.frozen}
        variant="inline"
      />
    </div>
  {:else}
    <PollNoticeCard
      poll_id={pn.poll_id}
      subject={pn.subject}
      outcome={pn.outcome}
      opener={pn.opener}
      {timestamp}
    />
  {/if}
{:else if "PseudocideRevival" in event.data}
  <PseudocideRevivalAnnouncement
    data={event.data.PseudocideRevival}
    {view}
    {timestamp}
  />
{:else if "KidnapReveal" in event.data}
  <KidnapRevealAnnouncement data={event.data.KidnapReveal} {view} {timestamp} />
{:else if "Kidnapping" in event.data}
  <KidnappingAnnouncement data={event.data.Kidnapping} {view} {timestamp} />
{:else if "Incarceration" in event.data}
  <IncarcerationAnnouncement
    data={event.data.Incarceration}
    {view}
    {timestamp}
  />
{:else if "IncarcerationReleased" in event.data}
  <IncarcerationReleasedAnnouncement
    data={event.data.IncarcerationReleased}
    {view}
    {timestamp}
  />
{:else if "NewIteration" in event.data}
  <NewIterationAnnouncement data={event.data.NewIteration} {view} {timestamp} />
{:else if "Blackout" in event.data}
  <BlackoutAnnouncement data={event.data.Blackout} {view} {timestamp} />
{:else if "ChannelTapped" in event.data}
  <ChannelTappedAnnouncement
    data={event.data.ChannelTapped}
    {view}
    {timestamp}
  />
{:else if "TapInResult" in event.data}
  <TapInResultAnnouncement data={event.data.TapInResult} {view} {timestamp} />
{:else if "FakeLoungeTapped" in event.data}
  <FakeLoungeTappedAnnouncement
    data={event.data.FakeLoungeTapped}
    {view}
    {timestamp}
  />
{:else if "KiraConnectionAttempt" in event.data}
  <KiraConnectionAttemptAnnouncement
    data={event.data.KiraConnectionAttempt}
    {view}
    {timestamp}
  />
{:else if "ContactLogEntry" in event.data}
  {@const log = event.data.ContactLogEntry}
  <ContactLogRow
    from={log.contactor}
    to={log.contacted}
    event={log.event}
    {timestamp}
    {view}
  />
{:else if "ProsecutionEvent" in event.data}
  <ProsecutionAnnouncement
    data={event.data.ProsecutionEvent}
    {view}
    {timestamp}
  />
{/if}

