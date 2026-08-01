use serde::{Deserialize, Serialize};

use crate::{
    ability::AbilityName,
    actor::{ActorDisplay, ActorKind, States, Statuses},
    bug::BugContext,
    channel::{ChannelKind, ChannelProfileView, ProfileOwners},
    common::{
        AbilityKey, ActorKey, AttemptCount, BugKey, ChannelKey, ChargeCount, GroupchatKey, ID,
        IncarcerationKey, IterationCount, KidnappingKey, LogID, NotebookKey, PassiveKey, PollKey,
        PollWeight, ProsecutionKey, Time, ViewportKey,
    },
    organization::OrganizationName,
    passive::{ContactLog, PassiveType},
    poll::{PollOptionIndex, PollOptionTally, PollOutcome, PollParent, PollSubject},
    prosecution::{ProsecutionPhaseView, ProsecutionSide},
    role::Role,
    viewport::ViewportKind,
};

// Every command is addressed. There is no "no recipient" case: a command that appears to be
// addressed to nobody is really addressed to some object's viewport, and the object decides who
// may read it.
//
// the frontend server is expected to intercept certain commands if they wish to implement host controls

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandRecipient {
    // The host itself, and by extension the admin's omniscient mirror. Used deliberately and
    // sparingly: world events that admin must see unredacted while players see a deception, and
    // per-player facts admin needs to inspect (RoleUpdate, TrueNameUpdate).
    System,
    // an actor (player or org) that already exists/is participating. for an org recipient,
    // the frontend gates visibility per player by their view of the org's channel.
    Actor(ActorKey),
    // Everyone with access to the viewport. An actor gaining access receives everything
    // previously addressed there, so this is what carries history to a late arrival.
    Viewport(ViewportKey),
    // The record, which is nobody. Nothing addressed here is ever delivered to any client, admin
    // included; it is written so the server can answer an autopsy or a tap-in later, from what was
    // actually said rather than from who happened to be listening.
    Log(LogID),
}

impl CommandRecipient {
    pub fn is_system(&self) -> bool {
        matches!(self, CommandRecipient::System)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    pub timestamp: Time,
    pub recipient: CommandRecipient,
    pub cmd: Command,
}

// What a tap-in guess turned up. The two misses read differently on purpose: a contact channel is
// loggable unless an admin deliberately turned it off, so "dark" is a rare and meaningful answer
// rather than a way of hiding whether the id was real.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TapInOutcome {
    // There is a record to read. The frontend server queries this log directly; `range` is how far
    // back from now, or None for everything it ever held. The log rather than the channel, so
    // answering a tap-in needs no model of which channel owns what.
    Found { log: LogID, range: Option<Time> },
    // Nothing has ever been registered under that id.
    NoSuchContact,
    // The channel is real, but logging is off there, so nothing was ever written down.
    NotLoggable,
}

// command the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    ////////////////////////////////////////////////
    // VIEWPORTS //
    ////////////////////////////////////////////////
    // Access changes, addressed to the actor whose access changed. The engine states them
    // rather than leaving them to be re-derived: a server that inferred access from
    // UpdateChannelView's permission bits would have to understand engine visibility rules,
    // and there is no equivalent carrier at all for notebooks or passives.
    //
    // Gaining access delivers everything previously addressed to the viewport, in order.
    // Losing access only stops further delivery — nothing already received is ever retracted.
    /// The actor may now read this viewport.
    EnterViewport {
        viewport: ViewportKey,
        actor: ActorKey,
    },

    /// The actor may no longer read this viewport. Their existing state stands.
    ExitViewport {
        viewport: ViewportKey,
        actor: ActorKey,
    },

    /// What kind of object this viewport belongs to, emitted when it is allocated and addressed
    /// to the viewport itself -- which is what makes an opaque key self-describing, and puts this
    /// at the head of every backfill, before any content can arrive on a viewport the recipient
    /// cannot yet name.
    ///
    /// Being addressed to the viewport, a client learns the kind exactly when it is admitted, and
    /// that is the intended gate: the kind is information, and knowing a viewport of some kind
    /// exists without being in it is a leak. It rides the viewport rather than an access grant so
    /// that the fact exists in the log for a reader that sees everything -- nobody is ever granted
    /// a Log viewport, so on a grant the frontend server could never recognise the one kind it
    /// must not forward.
    MapViewport {
        viewport: ViewportKey,
        kind: ViewportKind,
    },

    ////////////////////////////////////////////////
    // WORLD //
    ////////////////////////////////////////////////
    // World events are presence-gated notifications, addressed to the presence viewport (plus
    // a System mirror so admin sees them unredacted). How a client presents them — amane
    // renders them in the news channel — is entirely a client choice and no concern of the
    // protocol.

    // notify a specific player of a death. this can be done in any way. it can be put into the news
    // channel display, a dedicated list, etc...
    // the server doesn't need to intercept this because hosts can directly view the event log and
    // view/modify actor states
    Death {
        target_id: ActorKey,
        true_name: String,
        death_message: String,
        role: Role,
        notebook_transferred: bool,
        ability_transferred: bool,
    },

    // display/announce kidnapping. can be handled similar to death. The victim is public from the
    // start; the id lets clients track the kidnapping (e.g. a live timer) and lets the later reveal
    // reference it. duration is optional (indefinite kidnappings have none).
    Kidnapping {
        kidnapping_id: KidnappingKey,
        target_id: ActorKey,
        duration: Option<Time>,
    },

    // announce an incarceration. Mirrors Kidnapping, minus any reveal: an incarceration's source is
    // never disclosed, so who ordered it is not carried here and never follows.
    Incarceration {
        incarceration_id: IncarcerationKey,
        victim_id: ActorKey,
        duration: Option<Time>,
    },

    // the prisoner is out. Carries only the id -- the victim is resolved client-side from the
    // Incarceration that introduced it, exactly as KidnapReveal resolves its own.
    IncarcerationReleased {
        incarceration_id: IncarcerationKey,
    },

    // announce a kidnap reveal for a prior Kidnapping (referenced by id): either leaks the kidnapper
    // or shows none, meaning it was anonymous. The victim is resolved client-side from the id.
    KidnapReveal {
        kidnapping_id: KidnappingKey,
        kidnapper: Option<ActorKey>,
    },

    // display/announce a pseudocide revival. can be handled similarly to death.
    PseudocideRevival {
        target_id: ActorKey,
    },

    AnonymousAnnouncement {
        content: String,
    },

    // A silent prosecution named somebody who was not wanted, and the accuser is burned for it:
    // their true name is read out and the organization that has just barred them is named with it.
    //
    // Who they accused is deliberately absent. The world learns that an accusation was made and was
    // wrong, never who it was made against — being quietly cleared by somebody else's mistake is not
    // a thing you get handed in public.
    //
    // The org is carried by NAME rather than by key. An announcement everyone present receives has
    // to be readable by everyone present, and an org's actor key only resolves for a client that can
    // see that org's channel — which is to say, for its own members.
    FailedSilentProsecution {
        accuser_id: ActorKey,
        true_name: String,
        org: OrganizationName,
    },

    ////////////////////////////////////////////////
    // Actors //
    ////////////////////////////////////////////////
    // Actors will often have their state modified.
    // Both organizations and players are actors.

    // DIRECTED (to the actor itself): this actor's own current states. A client learns the
    // full state of the actors it holds and nothing else.
    //
    // There is deliberately no broadcast form. What OTHER viewers are allowed to know about an
    // actor is announced by the explicit event that caused it — Death, Kidnapping, Bugged, and
    // so on — and clients render other actors' status from those. A general "here is this
    // actor's state" broadcast would need a visibility rule of its own to answer a question
    // every one of those commands already answers, per viewer, correctly.
    ActorState {
        state: States,
        actor_id: ActorKey,
    },

    // BROADCAST (world-data viewport): what everyone may see of this actor's condition. The public
    // counterpart to ActorState — where that is the actor's own raw state set, this is the curated
    // projection others render them from, so it answers the visibility question once, for the whole
    // world, instead of leaving each event to.
    //
    // Rides world-data, so it survives a blackout: during one the presence-removing flags collapse
    // into `missing` (see Status), telling the world someone is gone without saying why. Re-emitted
    // whenever a contributing fact moves — a relevant state, a bug landing or being archived, or the
    // lights going out or coming back.
    ActorStatus {
        actor_id: ActorKey,
        status: Statuses,
    },

    // display a player as an org member. carries the org id, so the frontend keys the
    // update by it directly. addressed to the viewport of the org's backing channel — who may
    // see an org's roster is exactly who may see the org's channel.
    // this includes dead players and such as they are still considered org members
    AddOrgMember {
        player_id: ActorKey,
        org_id: ActorKey,
    },

    // remove from org member list
    RemoveOrgMember {
        player_id: ActorKey,
        org_id: ActorKey,
    },

    ////////////////////////////////////////////////
    // COMMS //
    ////////////////////////////////////////////////
    // Everything here is addressed to the channel's viewport, so a player added to a channel
    // after messages have already been sent receives them on entry — the frontend does not have
    // to arrange this, and the engine does not have to re-send anything.
    //
    // Channel-based objects (notebooks, groupchats, lounges) ride the same viewport as their
    // backing channel, so their views follow channel access automatically. Losing access stops
    // further content; it does not take back what was already delivered.

    // add a message to a channel
    //
    // The display rather than the profile it was sent through. A message is a thing that was said
    // by someone, and it goes into the record as one: the log stores these raw, and a profile key
    // read back later names an object that may have changed hands or stopped existing. The display
    // is what was actually shown to the room, and that cannot stop being true.
    //
    // It is also what lets the engine speak here at all, as ActorDisplay::System, without holding a
    // profile of its own.
    AddMessage {
        content: String,
        channel_id: ChannelKey,
        sender_display: ActorDisplay,
    },

    // register an actor slot and say what holds it. See ActorKind for what each carries.
    //
    // A player is addressed to the presence viewport like a world event, and emitted after the new
    // player has entered it. Their watermark starts at zero, so entry backfills every prior
    // MapActor — a player joining late is handed the whole roster, and an absent one learns of
    // arrivals when presence returns. No new visibility rule.
    //
    // An org is addressed to its own channel's viewport instead, so it reaches its members and
    // nobody else.
    MapActor {
        actor_id: ActorKey,
        kind: ActorKind,
    },

    // register a channel and say what it belongs to. See ChannelKind for the kinds and what each
    // carries.
    //
    // Always addressed to the channel's own membership viewport, which is what makes it the head
    // of that channel's history: a frontend learns the channel exists from the same place its
    // content will arrive, so nothing can be mis-filed against a viewport that merely mentioned
    // it. (UpdateProsecution names a trial channel, for instance, but rides presence.)
    MapChannel {
        channel_id: ChannelKey,
        kind: ChannelKind,
    },

    // the channel is finished: no further content will ever be addressed here. Everything
    // already received stands, and the frontend must handle the cascading effects on things
    // tied to the channel (notebooks, groupchats, lounges, etc...).
    //
    // This absorbed the old DeleteChannel. Nothing can be un-said, so there is no deletion to
    // express — only archival.
    ArchiveChannel {
        channel_id: ChannelKey,
    },

    // a channel's loggability status (whether messages here can be logged — autopsied,
    // relayed to bugs, …). A global channel property, not per-viewer. Emitted with the
    // channel's initial value on creation and again whenever it's toggled, so a viewer with
    // loggability control can see and flip the current state.
    SetChannelLoggable {
        channel_id: ChannelKey,
        loggable: bool,
    },

    NewBug {
        bug_key: BugKey,
    },

    AddBugMessage {
        bug_key: BugKey,
        display: ActorDisplay,
        content: String,
    },

    // the bug is no longer active; nothing further will be relayed through it. Absorbed the old
    // DeleteBug — a bug that "should never have existed" still relayed what it relayed, and the
    // people who saw that keep it.
    ArchiveBug {
        bug_key: BugKey,
    },

    // DIRECTED (to the bug's target): notify a player that they are under surveillance and
    // in what context (an explicit bug ability vs being held in custody). Deliberately omits
    // who planted it — the target learns *that* they're bugged, never *who* bugged them. The
    // owner side needs no equivalent: they simply receive the relayed AddBugMessage stream.
    Bugged {
        context: BugContext,
    },

    // a new day. structural rather than an announcement, so it rides the world-data viewport: it
    // reaches everyone present, and an absent player is handed it when they return rather than
    // told at the time. A blackout does not hide it — the clock is not news.
    NewIteration {
        iteration: IterationCount,
    },

    // The world has gone dark, or come back. Rides the world-data viewport, because a client that
    // could not be told the news had stopped would have no way to tell silence from nothing
    // happening — which is the one thing it must never get wrong.
    //
    // What was hidden is not summarised on the way out. Everything announced during the blackout
    // was addressed to the world-events viewport and is handed over in order on re-entry, so the
    // catch-up is the ordinary backfill and nothing has to be remembered here.
    Blackout {
        active: bool,
    },

    // update the owner status of a gc for a player
    GcOwnerStatus {
        owner: bool,
        gc_id: GroupchatKey,
    },

    // DIRECTED (to the member) + System: whether this player is an OG of this org.
    //
    // Personal info, addressed like a role or a true name — you know your own standing, admin can
    // inspect anyone's, and the rest of the org is told nothing. The org's ROSTER is a separate
    // thing that rides the org channel and says only who is in it.
    OgStatus {
        target_id: ActorKey,
        org_id: ActorKey,
        og: bool,
    },

    // DIRECTED: every name the room can currently see, and what each may do. The whole set, every
    // time, sent to each viewer when it changes and to anyone the moment they gain sight of the
    // channel.
    //
    // Directed and whole rather than addressed to the channel's viewport, and that is the entire
    // point. A viewport replays its history to anyone who enters, so a roster delivered that way
    // would hand every new arrival every name the channel has ever held — the previous holder of a
    // notebook, everyone who was ever in a lounge, every mask worn at a trial. A roster is current
    // state and not a sequence of events, so it is synchronised rather than logged.
    //
    // Invisible profiles are absent from it. Their existence is the thing being kept.
    ChannelRoster {
        channel_id: ChannelKey,
        profiles: Vec<ChannelProfileView>,
    },

    // SYSTEM only. The ownership behind a channel's roster: for every visible profile, which actors
    // wear it. Emitted alongside the System copy of ChannelRoster (both from cmd_channel_roster),
    // and only to System, because seeing through a name to the person behind it is exactly the
    // admin power ordinary viewers lack. Keyed by profile_id so it lines up with that roster.
    ProfileOwnership {
        channel_id: ChannelKey,
        owners: Vec<ProfileOwners>,
    },

    // DIRECTED: which profiles in this channel the recipient may speak as, whether or not the room
    // can see them, and what each permits.
    //
    // Says nothing about whether the recipient can READ the channel — that is the viewport's
    // answer and never this one. This is about which names are yours to use.
    //
    // An empty set is a member who holds nothing here, which is how losing your last name is
    // stated rather than left to be noticed.
    ProfileAccess {
        channel_id: ChannelKey,
        profiles: Vec<ChannelProfileView>,
    },

    // Somebody reached for Kira through this lounge. Addressed to the lounge's channel, and emitted
    // whether or not anyone was found — there is no quiet way to feel for Kira. `user` is raw and
    // always present: the point of the ability is that making the attempt costs you your anonymity
    // in that lounge. `success` says whether a Kira was actually on the other end, so the client
    // words it as a connection made or as nobody being there.
    KiraConnectionAttempt {
        channel_id: ChannelKey,
        user: ActorKey,
        success: bool,
    },

    ////////////////////////////////////////////////
    // NOTEBOOKS //
    ////////////////////////////////////////////////
    // Any notebook attempt should be shown to anybody who currently possesses the notebook.
    // The way this is handled doesn't matter.
    // This means that while one player may receive immediate feedback, other players should see
    // the previous attempts in the notebook.
    // This is not a command in of itself because the command would essentially be a null command
    // and would serve no purpose.
    // Note that messages sent in a notebook channel are handled by design. This specifically refers
    // to notebook usages which may be represented differently.
    //
    // Some modifiers block certain notebook actions. A frontend can take this into account.
    //
    // A write failure is not actually a failure to use an action. it is just the lack of a correct
    // true name and leads to actual state modification. the player must be explicitly notified, and
    // the usage must be logged. The viewability of writes is governed by the same rules as channel
    // messages.

    // notebook writes encompass everything the frontend could possibly need
    // the frontend should display all info when relevant
    NotebookWrite {
        notebook_id: NotebookKey,
        user_id: ActorKey,
        message: Option<String>,
        true_name: String,
        delay: Time,
        successes_remaining: AttemptCount,
        attempts_remaining: AttemptCount,
        success: bool,
        target_saved: bool,
    },

    // whether a notebook is currently on loan (being borrowed rather than truly owned). A
    // global notebook property (like a channel's loggability) — the frontend shows it in the
    // notebook channel. Deliberately doesn't say who lent it, just that it's borrowed.
    NotebookBorrowingStatus {
        notebook_id: NotebookKey,
        borrowed: bool,
    },

    // whether a notebook is fake (its writes cannot kill). Unlike the borrowing status — a fact
    // about whoever currently holds the book — this is told only to its ORIGINAL owner, mirrored to
    // System, the one entitled to know their own book is a decoy. Merely holding or even owning the
    // book earns nothing: a borrower, or someone who inherited it by killing the owner, is never
    // told, and deducing the decoy from a write that fails to kill is part of the game. Restated
    // whenever the flag changes (SetNotebookFake), otherwise stated once when the owner is set.
    NotebookFakeStatus {
        notebook_id: NotebookKey,
        fake: bool,
    },

    ////////////////////////////////////////////////
    // ABILITIES & PASSIVES //
    ////////////////////////////////////////////////
    // Clients may display some specific abilities differently from general abilities, but the
    // engine will have no knowledge of this. For instance, the contact ability should not be
    // treated as a normal ability on the frontend, but the engine sees it as no different than any
    // other ability.
    //
    // The specific actor that an ability belongs to should be taken into consideration.
    // As an example; even though you do not directly own organization abilities if you are in that
    // organization, it should still be clearly displayed, but differentiated from standard
    // self-owned abilities.
    // For this reason, there will be an owner id in the ability view command. If it is the client's
    // id, it doesn't really matter. If it is the org's id, it does.

    // One line in a contact log, addressed to the passive's own viewport — so gaining the passive
    // backfills everything it ever logged, exactly as gaining a channel does.
    //
    // passive_id names which log this belongs to: an actor can reach more than one contact-log
    // passive, and the two are separate records even where they overlap.
    //
    // Only the contactor's Modifier::LogNullification suppresses it. Being contacted by someone off
    // the record does not take your own contacts off it.
    AddContactLog {
        passive_id: PassiveKey,
        log: ContactLog,
    },

    // update the view of an ability to reflect its current state. usages are split by
    // outcome because conditional charge subtraction means successful and failed uses can
    // have different remaining counts (see Ability::get_ability_view_counts).
    UpdateAbilityView {
        ability_name: AbilityName,
        success_usages_remaining: ChargeCount,
        failure_usages_remaining: ChargeCount,
        iterations_to_reset: IterationCount,
        ability_id: AbilityKey,
        owner_id: ActorKey,
    },

    // entirely hide an ability from a user
    RemoveAbility {
        ability_id: AbilityKey,
    },

    // a passive the recipient now holds. Like UpdateAbilityView but with no charges/usages;
    // passive_type is the full typed value (some variants carry data, e.g. VoteAmplification's
    // multiplier). Doubles as create-and-reveal. Directed to the owner.
    UpdatePassiveView {
        passive_type: PassiveType,
        passive_id: PassiveKey,
        owner_id: ActorKey,
    },

    // hide a passive from the recipient (transferred away or destroyed). Directed to the
    // (former) owner.
    RemovePassive {
        passive_id: PassiveKey,
    },

    // tell the frontend to display autopsy messages. the frontend server will do the querying and
    // filtering, and the clients will handle the display of that info.
    //
    // `log` is the target's own record, naming them as the sender of everything they said wherever
    // they said it — which is what makes an autopsy answer for a message sent under a borrowed
    // display.
    RevealAutopsyMessages {
        log: LogID,
        range: Time,
        redact_names: bool,
    },

    // what a tap-in guess found, addressed to whoever guessed. every outcome is reported, because
    // learning that an id is unused is a real (and rationed) result, not an error.
    TapInResult {
        contact_id: ID,
        outcome: TapInOutcome,
    },

    // this channel was read by somebody outside the conversation. addressed to the channel, and
    // deliberately anonymous — the members learn they were tapped, never by whom. that gap is what
    // makes tapping a line you are on yourself a move rather than a waste.
    ChannelTapped {
        channel_id: ChannelKey,
    },

    // privately reveal a target player's true name to the recipient (BackgroundCheck)
    RevealTrueName {
        target_id: ActorKey,
        true_name: String,
    },

    // privately reveal whether a target is currently holding a notebook (NotebookReveal)
    RevealNotebookHolding {
        target_id: ActorKey,
        holding: bool,
    },

    ////////////////////////////////////////////////
    // PERSONAL INFO //
    ////////////////////////////////////////////////
    // A player's own identity facts, emitted when they change. Dual-routed: to the player
    // themselves (Actor(target)) so it lands in their notifications log ("your role is now
    // X"), and to System so admin can inspect any player's current facts per-user. target_id
    // is redundant for the player copy but is what keys the admin copy.
    RoleUpdate {
        target_id: ActorKey,
        role: Role,
    },

    TrueNameUpdate {
        target_id: ActorKey,
        true_name: String,
    },

    ////////////////////////////////////////////////
    // POLLS //
    ////////////////////////////////////////////////
    // Poll data is split: the shared part (subject, parent, tally) is addressed to the parent's
    // viewport via UpdatePoll; the per-player part (can I vote, what did I vote) rides a
    // directed UpdatePollView. The per-player split exists because a fresh client rebuilds
    // purely from the command stream — a player's own vote can't be tracked client-side
    // across a reconnect.
    //
    // Polls own no viewport. They ride the viewport of whatever they were put to, so a client
    // that reaches a parent's viewport replays every poll that parent ever held, concluded ones
    // included. That is intended: a concluded vote is part of what happened there.

    // create or refresh a poll's shared data, keyed by poll id. Re-sent on each vote change
    // to update the tally (counts only, never who voted).
    UpdatePoll {
        poll_id: PollKey,
        subject: PollSubject,
        parent: PollParent,
        // The choices and the weight behind each, in the order they are offered. Votes name an
        // option by its position here.
        options: Vec<PollOptionTally>,
        potential: PollWeight,
        // Who opened the vote (None = no distinct opener, e.g. a system-driven poll). Carried on
        // every update but only surfaced on the client's first-sight "vote started" notice.
        opener: Option<ActorKey>,
    },

    // a poll concluded. It closes, it is not dropped — outcome drives the resolution notice
    // rendered in the poll's parent location, and the closed poll stays visible to whoever
    // could see it.
    ClosePoll {
        poll_id: PollKey,
        outcome: PollOutcome,
    },

    // this player's personal view of a poll: whether they may currently vote, and the option
    // they've chosen (None until they cast one). Paired with membership of the parent's
    // viewport, which is what makes a player a viewer of the poll.
    UpdatePollView {
        poll_id: PollKey,
        eligible: bool,
        own_vote: Option<PollOptionIndex>,
    },

    ////////////////////////////////////////////////
    // PROSECUTIONS //
    ////////////////////////////////////////////////
    // The ordered timeline matters here (custody announcement → trial → verdict), and it is the
    // presence viewport that preserves it: a player who loses presence exits, and re-entry
    // backfills the whole gap in order. No special case, and no queue.
    //
    // The trial channel and verdict poll are NOT owned by this protocol — their contents are
    // addressed to the channel's and poll's own viewports; any divergence there is an engine
    // bug. UpdateProsecution does carry the trial channel id, but only so the frontend can tag
    // that channel as a prosecution channel and render it differently.
    //
    // Recipients: the presence viewport, plus a System mirror. The rigid "no recipient" vs
    // "targeted" split below no longer describes reality; a command's recipients are documented
    // per command from here on.

    // Create or refresh a prosecution's client-facing snapshot, keyed by prosecution id. Custody
    // doubles as the "someone is being prosecuted" announcement. trial_channel is None until the
    // trial channel exists, then names it so the frontend can render it as a prosecution channel.
    // Receiving one clears the frozen notice below.
    UpdateProsecution {
        prosecution_id: ProsecutionKey,
        prosecutor_display: ActorDisplay,
        defendant_display: ActorDisplay,
        phase: ProsecutionPhaseView,
        trial_channel: Option<ChannelKey>,
        // The defendant's chosen lawyer, once they have one. Public: a trial's defense counsel is
        // not a hidden fact, and it is the same kind of thing as the two displays above.
        //
        // NOT accompanied by the lawyer's private channel, which is addressed to its own viewport
        // and reaches only the two of them -- see ChannelKind::Lawyer.
        lawyer_display: Option<ActorDisplay>,
    },

    // DIRECTED (to the participant named by `side`): which side of this prosecution you are on.
    //
    // The snapshot above cannot carry this. An anonymous prosecutor's display is Mysterious in the
    // one UpdateProsecution everybody receives — including the copy the prosecutor themselves
    // receives — so without this they could not tell their own trial from anyone else's, and
    // putting their identity in the snapshot to fix that would tell the whole world. The defendant
    // is currently always shown raw and learns nothing new from this, and is sent it anyway: one
    // rule for both sides, so the client never has to know which displays happen to be anonymous
    // today.
    //
    // Carries no identity of its own. It names a prosecution the recipient already receives the
    // public facts about and adds exactly one private fact to it, which is why it leaks nothing
    // even though the whole point is that it is about who someone is.
    //
    // Not viewport-addressed. Being a party to a prosecution is a standing fact rather than an
    // event, so it is never revoked and never withheld — a blackout stops the trial's news from
    // reaching you, not your knowledge of whose trial it is.
    InProsecution {
        prosecution_id: ProsecutionKey,
        side: ProsecutionSide,
    },

    // The prosecution ended (verdict reached, terminated, etc.). Addressed the same way as
    // UpdateProsecution, so for an absent player it lands after any pending updates when they
    // return.
    CloseProsecution {
        prosecution_id: ProsecutionKey,
        // Some(true) guilty, Some(false) acquitted. None when the prosecution ended without ever
        // reaching a verdict — terminated by a host, or an invariant broke (a participant lost
        // presence, the source ability was destroyed).
        //
        // An acquittal has no other trace: nobody dies, so this is the only thing that says it
        // happened.
        verdict: Option<bool>,
    },
}
