use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::{
    ability::AbilityName,
    actor::{ActorDisplay, ActorKind, States},
    bug::BugContext,
    channel::{ChannelKind, ChannelPermissions},
    common::{
        AbilityKey, ActorKey, AttemptCount, BugKey, ChannelKey, ChargeCount, GroupchatKey, ID,
        IncarcerationKey, IterationCount, KidnappingKey, NotebookKey, PassiveKey, PollKey,
        PollWeight, ProsecutionKey, Time, ViewportKey,
    },
    passive::{ContactLog, PassiveType},
    poll::{PollOutcome, PollSubject, PollVisibility},
    prosecution::ProsecutionPhaseView,
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
    // There is a record to read. The frontend server queries this channel's log viewport; `range`
    // is how far back from now, or None for everything it ever held.
    Found {
        channel_id: ChannelKey,
        range: Option<Time>,
    },
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

    // a new day. a world event, so it reaches everyone present — and, like every world event, an
    // absent player is handed it when they return rather than told at the time.
    NewIteration {
        iteration: IterationCount,
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

    // display a channel member
    ShowChannelMember {
        channel_id: ChannelKey,
        display: ActorDisplay,
        channel_perms: ChannelPermissions,
    },

    // remove a channel member display
    RemoveChannelMember {
        channel_id: ChannelKey,
        display: ActorDisplay,
    },

    // update a player's view of the channel based on their permissions
    UpdateChannelView {
        channel_id: ChannelKey,
        perms: ChannelPermissions,
        displays: IndexSet<ActorDisplay>,
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

    // tell the frontend to display autopsy messages for a specific user. the frontend server will do the
    // querying and filtering, and the clients will handle the display of that info.
    RevealAutopsyMessages {
        target_id: ActorKey,
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
    // Poll data is split: the shared part (subject, scope, tally) is addressed to the poll's
    // viewport via UpdatePoll; the per-player part (can I vote, what did I vote) rides a
    // directed UpdatePollView. The per-player split exists because a fresh client rebuilds
    // purely from the command stream — a player's own vote can't be tracked client-side
    // across a reconnect.

    // create or refresh a poll's shared data, keyed by poll id. Re-sent on each vote change
    // to update the tally (counts only, never who voted).
    UpdatePoll {
        poll_id: PollKey,
        subject: PollSubject,
        scope: PollVisibility,
        accept: PollWeight,
        reject: PollWeight,
        potential: PollWeight,
        // Who opened the vote (None = no distinct opener, e.g. a system-driven poll). Carried on
        // every update but only surfaced on the client's first-sight "vote started" notice.
        opener: Option<ActorKey>,
    },

    // a poll concluded. It closes, it is not dropped — outcome drives the resolution notice
    // rendered in the poll's scoped location, and the closed poll stays visible to whoever
    // could see it.
    ClosePoll {
        poll_id: PollKey,
        outcome: PollOutcome,
    },

    // this player's personal view of a poll: whether they may currently vote, and the vote
    // they've cast (None until they cast one). Paired with EnterViewport on the poll's
    // viewport, which is what actually makes a player a viewer of the poll.
    UpdatePollView {
        poll_id: PollKey,
        eligible: bool,
        own_vote: Option<bool>,
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
