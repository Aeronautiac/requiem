use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::{
    ability::AbilityName,
    actor::{ActorDisplay, States},
    bug::BugContext,
    channel::ChannelPermissions,
    common::{
        AbilityKey, ActorKey, AttemptCount, BugKey, ChannelKey, ChargeCount, GroupchatKey, ID,
        IterationCount, KidnappingKey, LoungeKey, NotebookKey, PassiveKey, PollKey, PollWeight,
        ProsecutionKey, Time, ViewportKey,
    },
    organization::OrganizationName,
    passive::PassiveType,
    poll::{PollOutcome, PollSubject, PollVisibility},
    prosecution::ProsecutionPhaseView,
    role::Role,
    viewport::ViewportKind,
    world::WorldChannelName,
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
    /// The actor may now read this viewport. `kind` is a display aid only; nothing may
    /// branch on it.
    EnterViewport {
        viewport: ViewportKey,
        actor: ActorKey,
        kind: ViewportKind,
    },

    /// The actor may no longer read this viewport. Their existing state stands.
    ExitViewport {
        viewport: ViewportKey,
        actor: ActorKey,
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

    // register a player: the raw actor slot exists, and that is the whole of what the engine has
    // to say about it. No presentation rides here — a display name is a server-level fact about
    // WHO is playing the slot, with a different lifetime, and it arrives on its own channel.
    // (`true_name` is deliberately not it: that is a MECHANIC, secret, and the thing written in a
    // notebook.)
    //
    // Addressed to the presence viewport like a world event, and emitted after the new player has
    // entered it. Their watermark starts at zero, so entry backfills every prior MapPlayer — a
    // player joining late is handed the whole roster, and an absent one learns of arrivals when
    // presence returns. No new visibility rule.
    MapPlayer {
        player_id: ActorKey,
    },

    // map a lounge id to a channel id. contact_id is the lounge's strictly-increasing
    // contact-channel id, used for display (e.g. "lounge-<contact_id>") and to reference the
    // contact channel (tap-ins, contact logs).
    MapLounge {
        lounge_id: LoungeKey,
        channel_id: ChannelKey,
        contact_id: ID,
    },

    // map a gc id to a channel id. contact_id as in MapLounge (rendered like "<name> [<contact_id>]";
    // no custom names yet, so a default name for now).
    MapGc {
        gc_id: GroupchatKey,
        channel_id: ChannelKey,
        contact_id: ID,
    },

    // register an org on the frontend: its actor id, name, and backing channel (and any
    // future org-level data). one unified command, addressed like the other channel maps.
    MapOrg {
        org_id: ActorKey,
        channel_id: ChannelKey,
        org_name: OrganizationName,
    },

    // there is only one instance of every world channel. a frontend must keep this in mind.
    MapWorldChannel {
        channel_id: ChannelKey,
        channel_name: WorldChannelName,
    },

    // register a personal channel: a plain engine channel a player created for themselves
    // (a notepad / a private line to whoever bugged them). Addressed to the channel's own
    // viewport like the other channel maps, so only the owner ever sees it. Sent so the
    // frontend can tag it as a personal channel.
    MapPersonalChannel {
        channel_id: ChannelKey,
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

    // update the owner status of a gc for a player
    GcOwnerStatus {
        owner: bool,
        gc_id: GroupchatKey,
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

    // map a notebook id to its channel id
    // the state of the display for a given player should depend on that player's permissions in the
    // notebook's channel
    MapNotebook {
        notebook_id: NotebookKey,
        channel_id: ChannelKey,
    },

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

    // similarly to channels, when someone gets access to a contact log passive, they should be able
    // to see EVERYTHING previously logged by that specific passive.
    // for this, use passive ids.
    // contact logs include group chat additions and such as well
    AddContactLog {
        // log: ContactLog,
        passive_id: PassiveKey,
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
    },

    // The prosecution ended (verdict reached, terminated, etc.). Addressed the same way as
    // UpdateProsecution, so for an absent player it lands after any pending updates when they
    // return.
    CloseProsecution {
        prosecution_id: ProsecutionKey,
    },
}
