// Dispatches a single non-message event to its own rendering branch. Each branch knows how to draw
// its one event type as an Announcement (or its dedicated card), so ChannelView stays a short
// dispatch rather than a long inline chain of its own.
import type { BugContext, Role, TapInOutcome } from "amane-client/bindings.ts";
import type { GameEvent, PollData, PollView } from "amane-client/game/types.ts";
import type { View } from "amane-client/game/view.ts";
import {
  formatDuration,
  nameLabel,
  orgDisplayName,
  phaseAnnouncementKey,
  refFromDisplay,
  refFromKey,
  roleLabel,
  t,
} from "amane-client/text.ts";
import { Template } from "../../kit/Template.tsx";
import { roleColorVar } from "../../style.ts";
import { Chip, Entity } from "../Name.tsx";
import { MentionText } from "../mentions/MentionText.tsx";
import { Announcement } from "./Announcement.tsx";
import { ContactLogRow } from "./ContactLogRow.tsx";
import { PollNoticeCard } from "./PollNoticeCard.tsx";
// Another agent's panel: the live, votable card. Assumed props per the shared UI brief.
import { PollCard } from "../side/PollCard.tsx";

// A poll's start notice rides its home channel's stream. While the poll is still live we render the
// interactive card in place of the "vote started" announcement; once it resolves, the announcement
// (with its outcome) takes over again.
function livePoll(view: View, poll_id: string): { data: PollData; pollView: PollView | null; frozen: boolean } | null {
  const data = view.polls.get(poll_id);
  if (!data || data.outcome) return null;
  return { data, pollView: view.poll_views.get(poll_id) ?? null, frozen: view.frozen(data.viewport) };
}

// success = the name matched a real player; target_saved = the kill didn't land.
function writeOutcomeText(w: { success: boolean; target_saved: boolean; delay: number; message: string; successes_remaining: number; attempts_remaining: number }): string {
  const lines: string[] = [];
  if (!w.success) lines.push("Outcome: Failure. There is nobody with that true name.");
  else if (w.target_saved) lines.push("Outcome: Something saved them...");
  else if (w.delay > 0) lines.push(`Outcome: Success. The target will die in ${formatDuration(w.delay)}.`);
  else lines.push("Outcome: Success. The target dies immediately.");
  if (w.message) lines.push(`Cause of death: ${w.message}`);
  lines.push(`Successes left: ${w.successes_remaining} · Attempts left: ${w.attempts_remaining}`);
  return lines.join("\n");
}

// Red = lethal, amber = valid-but-saved, grey = no match.
function writeColor(w: { success: boolean; target_saved: boolean }): string {
  if (!w.success) return "var(--color-event-nothing)";
  if (w.target_saved) return "var(--color-event-alarm)";
  return "var(--color-event-death)";
}

// A miss says WHICH miss on purpose: a contact channel is loggable unless an admin turned it off, so
// hitting a dark one is a real finding rather than a polite way of saying the number was wrong.
function tapInText(contact_id: number, outcome: TapInOutcome): string {
  if (outcome === "NoSuchContact") return `Contact ${contact_id} does not exist.`;
  if (outcome === "NotLoggable") return `Contact ${contact_id} exists, but is not loggable.`;
  const scope = outcome.Found.range === null ? "everything ever sent there" : `the last ${formatDuration(outcome.Found.range)}`;
  return `Tapped into contact ${contact_id}. Revealing ${scope}.`;
}

export function EventAnnouncement({ event, view, timestamp }: { event: GameEvent; view: View; timestamp: number }) {
  const data = event.data;

  if ("Write" in data) {
    const w = data.Write;
    return (
      <Announcement timestamp={timestamp} color={writeColor(w)} description="Notebook Write">
        <Entity of={{ kind: "player", id: w.user_id }} view={view} chip /> wrote the name <span className="text-ink">"{nameLabel(w.true_name)}"</span>.
        <div className="mt-1 whitespace-pre-wrap">{writeOutcomeText(w)}</div>
      </Announcement>
    );
  }

  if ("Death" in data) {
    const d = data.Death;
    return (
      // Beat 1 of the staged reveal: the death and the name behind it. The role and any inheritance
      // follow as their own events (DeathRole / DeathTransfer), timed apart.
      <Announcement timestamp={timestamp} color="var(--color-event-death)" description="Death">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> has died.
        {d.death_message && (
          <div className="mt-1 italic whitespace-pre-wrap text-ink">
            <MentionText content={d.death_message} view={view} />
          </div>
        )}
        <div className="mt-1 text-ink-dim">
          Their true name was <span className="text-ink">{nameLabel(d.true_name)}</span>.
        </div>
      </Announcement>
    );
  }

  if ("DeathRole" in data) {
    const d = data.DeathRole;
    return (
      // Beat 2: who they turned out to be, in that role's own colour.
      <Announcement timestamp={timestamp} color={roleColorVar(d.role)} description="Role Revealed">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> was <Chip label={roleLabel(d.role)} colorVar={roleColorVar(d.role)} />.
      </Announcement>
    );
  }

  if ("DeathOrgs" in data) {
    const d = data.DeathOrgs;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-org-reveal)" description="Affiliations">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> stood with{" "}
        {d.orgs.map((org, i) => {
          return (
            <span key={org.id}>
              <Entity of={refFromKey(org.id, view.players, view.orgs)} view={view} chip />
              {org.leader ? <span className="text-ink-dim"> (leader)</span> : org.og ? <span className="text-ink-dim"> (OG)</span> : null}
              {i < d.orgs.length - 1 ? ", " : null}
            </span>
          );
        })}
        .
      </Announcement>
    );
  }

  if ("DeathTransfer" in data) {
    const d = data.DeathTransfer;
    const text =
      d.notebook_transferred && d.ability_transferred
        ? "Their notebook(s) and their transferrable abilities have"
        : d.notebook_transferred
          ? "Their notebook(s) have"
          : "Their transferrable abilities have";
    return (
      // Beat 3: what they left behind.
      <Announcement timestamp={timestamp} color="var(--color-event-death)" description="Inheritance">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> had some notable possessions. {text} been given to the person responsible for their death.
      </Announcement>
    );
  }

  if ("AnonymousAnnouncement" in data) {
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-anonymous)" description="Anonymous Announcement">
        <div className="whitespace-pre-wrap">
          <MentionText content={data.AnonymousAnnouncement.content} view={view} />
        </div>
      </Announcement>
    );
  }

  if ("EyeDealTaken" in data) {
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-reveal)" description="Shinigami Eye Deal">
        <Entity of={refFromDisplay(data.EyeDealTaken.user, view.orgs)} view={view} chip /> has taken the shinigami eye deal.
      </Announcement>
    );
  }

  if ("NewsAnchor" in data) {
    const target_id = data.NewsAnchor.target_id;
    return (
      <Announcement timestamp={timestamp} color="var(--color-news-anchor)" description="News Anchor">
        {target_id ? (
          <>
            <Entity of={{ kind: "player", id: target_id }} view={view} chip /> is now the <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
          </>
        ) : (
          <>
            The <Chip label="News Anchor" colorVar="var(--color-news-anchor)" /> position is now vacant.
          </>
        )}
      </Announcement>
    );
  }

  if ("NewsAnchorStatus" in data) {
    const holding = data.NewsAnchorStatus.holding;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-personal)" description="News Anchor">
        You are {holding ? "now" : "no longer"} the <Chip label="News Anchor" colorVar="var(--color-news-anchor)" />.
      </Announcement>
    );
  }

  if ("PressConfMembership" in data) {
    const in_conf = data.PressConfMembership.in_conf;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-personal)" description="Press Conference">
        You {in_conf ? "joined" : "left"} the <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
      </Announcement>
    );
  }

  if ("LeaderStatus" in data) {
    const d = data.LeaderStatus;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-personal)" description="Leadership">
        {d.leader ? "You are now the leader of" : "You are no longer the leader of"}{" "}
        <Entity of={refFromKey(d.org_id, view.players, view.orgs)} view={view} chip />
        .
      </Announcement>
    );
  }

  if ("PressConfStatus" in data) {
    const d = data.PressConfStatus;
    return (
      <Announcement timestamp={timestamp} color="var(--color-press-conference)" description="Press Conference">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> {d.has_access ? "joined the" : "left the"}{" "}
        <Chip label="Press Conference" colorVar="var(--color-press-conference)" />.
      </Announcement>
    );
  }

  if ("FailedSilentProsecution" in data) {
    const d = data.FailedSilentProsecution;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-prosecution)" description="Failed Silent Prosecution">
        <Entity of={{ kind: "player", id: d.accuser_id }} view={view} chip /> has attempted to execute an innocent person of HEINOUS crimes...
        <div className="mt-1">
          True name: <span className="text-ink">{nameLabel(d.true_name)}</span>
        </div>
        <div className="mt-1">{orgDisplayName(d.org)} will NOT accept this, and so, they've been permanently banned from the organization.</div>
      </Announcement>
    );
  }

  if ("RevealTrueName" in data) {
    const d = data.RevealTrueName;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-reveal)" description="Name Reveal">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip />'s true name is <span className="text-ink">{nameLabel(d.true_name)}</span>.
      </Announcement>
    );
  }

  if ("RevealNotebookHolding" in data) {
    const d = data.RevealNotebookHolding;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-reveal)" description="Notebook Check">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> is {d.holding ? "" : "not "}currently holding a notebook.
      </Announcement>
    );
  }

  if ("EyeCount" in data) {
    const c = data.EyeCount.count;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-personal)" description="Shinigami Eyes">
        You have <span className="text-ink">{c}</span> eye{c === 1 ? "" : "s"} remaining.
      </Announcement>
    );
  }

  if ("Bugged" in data) {
    const context: BugContext = data.Bugged.context;
    return (
      <Announcement
        timestamp={timestamp}
        color="var(--color-event-surveillance)"
        description="Bugged"
        content={
          context === "Custody"
            ? "You are bugged. Your messages are being monitored while you are in custody."
            : "You have been bugged. Your messages are being monitored."
        }
      />
    );
  }

  if ("RoleUpdate" in data) {
    const role: Role = data.RoleUpdate.role;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-personal)" description="Role">
        Your role is now <Chip label={roleLabel(role)} colorVar={roleColorVar(role)} />.
      </Announcement>
    );
  }

  if ("TrueNameUpdate" in data) {
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-personal)" description="True Name">
        Your true name is now <strong>{nameLabel(data.TrueNameUpdate.true_name)}</strong>.
      </Announcement>
    );
  }

  if ("NotebookReceived" in data) {
    return <Announcement timestamp={timestamp} color="var(--color-event-death)" description="Notebook" content="A notebook has come into your possession." />;
  }

  if ("PollNotice" in data) {
    const pn = data.PollNotice;
    const live = pn.outcome ? null : livePoll(view, pn.poll_id);
    if (live) {
      return (
        <div className="px-3 py-1" data-poll-anchor={pn.poll_id}>
          <PollCard id={pn.poll_id} data={live.data} pollView={live.pollView} frozen={live.frozen} variant="inline" />
        </div>
      );
    }
    return <PollNoticeCard poll_id={pn.poll_id} subject={pn.subject} outcome={pn.outcome} opener={pn.opener} timestamp={timestamp} view={view} />;
  }

  if ("Revival" in data) {
    const d = data.Revival;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-revival)" description="Revival">
        {/* Neutral on purpose: the engine's message below says how (divine intervention, a staged
            death), so the headline must not presume which. */}
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> has been revived.
        {d.message && (
          <div className="mt-1 italic whitespace-pre-wrap text-ink">
            <MentionText content={d.message} view={view} />
          </div>
        )}
      </Announcement>
    );
  }

  if ("KidnapReveal" in data) {
    const d = data.KidnapReveal;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-alarm)" description="Kidnap Reveal">
        Authorities have recovered{" "}
        {d.victim ? <Entity of={{ kind: "player", id: d.victim }} view={view} chip /> : "the victim"}. When the victim was questioned,{" "}
        {d.kidnapper ? (
          <>
            they revealed <Entity of={{ kind: "player", id: d.kidnapper }} view={view} chip /> as their kidnapper.
          </>
        ) : (
          "they could not identify their kidnapper(s)."
        )}
      </Announcement>
    );
  }

  if ("Kidnapping" in data) {
    const d = data.Kidnapping;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-alarm)" description="Kidnapping">
        <Entity of={{ kind: "player", id: d.target_id }} view={view} chip /> has been kidnapped. Authorities have begun rescue operations, but it may be a while before they are found.
      </Announcement>
    );
  }

  if ("Incarceration" in data) {
    const d = data.Incarceration;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-custody)" description="Imprisonment">
        <Entity of={{ kind: "player", id: d.victim_id }} view={view} chip /> has been imprisoned {d.duration ? `for ${formatDuration(d.duration)}` : ""}.
      </Announcement>
    );
  }

  if ("IncarcerationReleased" in data) {
    const victim = data.IncarcerationReleased.victim;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-custody)" description="Release">
        {victim ? <Entity of={{ kind: "player", id: victim }} view={view} chip /> : "A prisoner"} has been released.
      </Announcement>
    );
  }

  if ("NewIteration" in data) {
    const iteration = data.NewIteration.iteration;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-alarm)" description={iteration === 1 ? "The Game Begins" : "New Day"}>
        {iteration === 1 ? (
          <>
            <strong>Day 1</strong>. Various interactions have now been unlocked.
          </>
        ) : (
          <>
            <strong>Day {iteration}</strong>. Cooldowns have been progressed, and statuses have been cleared.
          </>
        )}
      </Announcement>
    );
  }

  if ("Blackout" in data) {
    const active = data.Blackout.active;
    return (
      <Announcement
        timestamp={timestamp}
        color="var(--color-event-blackout)"
        description={active ? "Blackout" : "Blackout Over"}
        content={active ? "The world's gone dark..." : "The light returns..."}
      />
    );
  }

  if ("ChannelTapped" in data) {
    return (
      <Announcement
        timestamp={timestamp}
        color="var(--color-event-surveillance)"
        description="Tap In"
        content="This channel has been tapped into. A log of the channel up to this point has been sent to the tapper."
      />
    );
  }

  if ("TapInResult" in data) {
    const d = data.TapInResult;
    const color = typeof d.outcome === "string" ? "var(--color-event-nothing)" : "var(--color-event-tap)";
    return <Announcement timestamp={timestamp} color={color} description="Tap In" content={tapInText(d.contact_id, d.outcome)} />;
  }

  if ("FakeLoungeTapped" in data) {
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-surveillance)" description="Fake Lounge Read">
        Your fake lounge was tapped into by <Entity of={refFromDisplay(data.FakeLoungeTapped.display, view.orgs)} view={view} chip />.
      </Announcement>
    );
  }

  if ("KiraConnectionAttempt" in data) {
    const d = data.KiraConnectionAttempt;
    return (
      <Announcement timestamp={timestamp} color={d.success ? "var(--color-event-death)" : "var(--color-event-nothing)"} description="Kira Connection">
        <Entity of={{ kind: "player", id: d.user }} view={view} chip /> has attempted to connect with <Chip label={roleLabel("Kira")} colorVar={roleColorVar("Kira")} />{" "}
        {d.success ? "and it was a success." : "They failed..."}
      </Announcement>
    );
  }

  if ("ContactLogEntry" in data) {
    const log = data.ContactLogEntry;
    return <ContactLogRow from={log.contactor} to={log.contacted} event={log.event} timestamp={timestamp} view={view} />;
  }

  if ("ProsecutionEvent" in data) {
    const d = data.ProsecutionEvent;
    return (
      <Announcement timestamp={timestamp} color="var(--color-event-prosecution)" description={d.ended ? "Prosecution Ended" : "Prosecution"}>
        <Template
          text={t(phaseAnnouncementKey(d.phase, d.ended, d.verdict))}
          parts={{
            prosecutor: <Entity of={refFromDisplay(d.prosecutor_display, view.orgs)} view={view} chip />,
            defendant: <Entity of={refFromDisplay(d.defendant_display, view.orgs)} view={view} chip />,
          }}
        />
      </Announcement>
    );
  }

  return null;
}
