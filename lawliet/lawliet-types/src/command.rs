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
        ProsecutionKey, Time,
    },
    organization::OrganizationName,
    passive::PassiveType,
    poll::{PollOutcome, PollSubject, PollVisibility},
    prosecution::ProsecutionPhaseView,
    role::Role,
    world::WorldChannelName,
};

// commands with no recipient are considered "system" commands and are used to talk directly to the
// host or the backend of a frontend
//
// the frontend server is expected to intercept certain commands if they wish to implement host controls

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandRecipient {
    System,
    BasePlayer, // any player shall be fed these commands, even if the player was created AFTER the
    // command was initially sent
    // an actor (player or org) that already exists/is participating. for an org recipient,
    // the frontend gates visibility per player by their view of the org's channel.
    Actor(ActorKey),
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
    // WORLD //
    ////////////////////////////////////////////////
    // if doing something like sending as a message in the news channel, ensure that it is placed
    // into the proper slot and treated as a historical event if it wasn't sent immediately.
    // also note that in the case of being rendered within a channel, and the channel not being available,
    // the message must be rendered regardless of the player's permissions within that channel
    // some "channels" should have something rendered regardless of if the player is even a member
    // of the channel. for instance, news should be treated as a special instance where it is both a
    // channel (if member), and an event log.

    /////=<TARGETTED>=/////

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
    // Actors will often have their state modified. Views of that actor should reflect
    // their current state(s).
    // Furthermore, there needs to be a way to address the underlying actor object of a player or org from
    // the frontend.
    // Both organizations and players are actors.

    /////=<NO RECIPIENT>=/////

    // all display instances of this actor must be updated
    // this is handled by the frontend
    // ensure that clients cannot see the state of other actors which are not visible to them
    // a clean way to handle this is to construct a set of display "blueprint" type objects on the
    // frontend server, map them to actors, and then send them out to every client
    // who currently has permission to view that actor in some way (when necessary)
    // the reason this isnt handled entirely on the engine level is because its irrelevant. there
    // are no deception mechanics regarding state displays.
    ActorState {
        state: States,
        actor_id: ActorKey,
    },

    // display a player as an org member. carries the org id, so the frontend keys the
    // update by it directly. the org member list is currently the same for everyone (no
    // per-viewer variation yet), so this is undirected rather than a per-member broadcast.
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

    /////=<TARGETTED>=/////

    ////////////////////////////////////////////////
    // COMMS //
    ////////////////////////////////////////////////
    // A player who is added to a channel after messages have already been sent should be allowed to
    // see the messages which have been sent in that channel previously if they have view
    // permissions. This must be handled by the frontend.
    // Channel based object views are dependent on channel views. If channel access is lost, the
    // object view must also be lost. (Notebooks, groupchats, lounges, etc...)

    /////=<NO RECIPIENT>=/////

    // add a message to a channel
    AddMessage {
        content: String,
        channel_id: ChannelKey,
        sender_display: ActorDisplay,
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
    // future org-level data). one unified command; global, like the other channel maps.
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
    // (a notepad / a private line to whoever bugged them). Like the other channel maps this
    // is global; only the owner holds perms for it, so per-viewer visibility falls out of the
    // normal channel-view perms. Sent so the frontend can tag it as a personal channel.
    MapPersonalChannel {
        channel_id: ChannelKey,
    },

    // delete a channel
    // the frontend must handle the cascading effects of handling things tied to the channel
    // (notebooks, groupchats, lounges, etc...)
    DeleteChannel {
        channel_id: ChannelKey,
    },

    // can no longer send messages or similar, but you can still view if you have/are given view permissions
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

    // shouldnt really do much. itll just say that the bug is no longer active.
    ArchiveBug {
        bug_key: BugKey,
    },

    // identical to setting visibility to false for everyone in the game - collapsed into a single
    // instruction
    ClearBugVisibily {
        bug_id: BugKey,
    },

    // completely destroy a bug (hide all views)
    // basically, this bug should have never existed
    DeleteBug {
        bug_id: BugKey,
    },

    // DIRECTED (to the bug's target): notify a player that they are under surveillance and
    // in what context (an explicit bug ability vs being held in custody). Deliberately omits
    // who planted it — the target learns *that* they're bugged, never *who* bugged them. The
    // owner side needs no equivalent: they simply receive the relayed AddBugMessage stream.
    Bugged {
        context: BugContext,
    },

    /////=<TARGETTED>=/////

    // remove someone's view of a channel
    RemoveChannel {
        channel_id: ChannelKey,
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

    SetBugVisibility {
        bug_id: BugKey,
        visible: bool,
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

    /////=<NO RECIPIENT>=/////

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

    /////=<TARGETTED>=/////

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

    /////=<NO RECIPIENT>=/////

    // similarly to channels, when someone gets access to a contact log passive, they should be able
    // to see EVERYTHING previously logged by that specific passive.
    // for this, use passive ids.
    // contact logs include group chat additions and such as well
    AddContactLog {
        // log: ContactLog,
        passive_id: PassiveKey,
    },

    /////=<TARGETTED>=/////

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
    // Poll data is split: the heavy, shared part (subject, scope, tally) is held globally
    // on the frontend via UpdatePoll; the lightweight per-player part (can I vote, what did
    // I vote) rides a directed UpdatePollView. The per-player split exists because a fresh
    // client rebuilds purely from the command stream — a player's own vote can't be tracked
    // client-side across a reconnect.

    /////=<NO RECIPIENT>=/////

    // create or refresh a poll's shared data. Held globally, keyed by poll id. Re-sent on
    // each vote change to update the tally (counts only, never who voted).
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

    // a poll concluded; the frontend drops it (globally and from every view). outcome
    // drives the resolution notice rendered in the poll's scoped location.
    ClosePoll {
        poll_id: PollKey,
        outcome: PollOutcome,
    },

    /////=<TARGETTED>=/////

    // this player's personal view of a poll: whether they may currently vote, and the vote
    // they've cast (None until they cast one). Directed to players who can see the poll's
    // scope; receiving one is what makes a player a "viewer" of the poll.
    UpdatePollView {
        poll_id: PollKey,
        eligible: bool,
        own_vote: Option<bool>,
    },

    // a viewer can no longer see the poll's scope: hide the poll for this player. The
    // frontend can't reliably decide this itself — some scopes (e.g. "present") it has no
    // notion of, and re-deriving the rest from channel membership would be brittle — so the
    // engine tracks who it sent poll data to and directs a removal when access is lost.
    RemovePollView {
        poll_id: PollKey,
    },

    ////////////////////////////////////////////////
    // PROSECUTIONS //
    ////////////////////////////////////////////////
    // Unlike polls, prosecution updates are never dropped when a player loses visibility: the
    // ordered timeline matters (custody announcement → trial → verdict), so absent players
    // receive the whole sequence deferred, replayed in order when presence returns. The trial
    // channel and verdict poll are NOT owned by this protocol — their contents ride the channel
    // and poll command streams respectively; any divergence there is an engine bug. UpdateProsecution
    // does carry the trial channel id, but only so the frontend can tag that channel as a
    // prosecution channel and render it differently.

    // Recipients: sent to everyone — System and BasePlayer immediately, and each existing player
    // too, with only the players receiving it deferred (held while they lack presence and replayed
    // in order on return). The rigid "no recipient" vs "targeted" split below no longer describes
    // reality; a command's recipients are documented per command from here on.

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

    // The prosecution ended (verdict reached, terminated, etc.); the frontend drops it. Sent to
    // everyone the same way as UpdateProsecution, so for absent players it lands (deferred) after
    // any pending updates.
    CloseProsecution {
        prosecution_id: ProsecutionKey,
    },

    // Directed to a single player who was receiving live updates but has lost presence: they are
    // now viewing frozen state. Purely a UI notice — the real updates are still queued and will
    // replay in order on return (an UpdateProsecution is what clears this).
    FreezeProsecutionView {
        prosecution_id: ProsecutionKey,
    },
}
