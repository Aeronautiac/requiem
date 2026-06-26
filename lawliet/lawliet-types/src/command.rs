use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::{
    ability::AbilityName,
    actor::{ActorDisplay, States},
    channel::ChannelPermissions,
    common::{
        AbilityKey, ActorKey, AttemptCount, BugKey, ChannelKey, ChargeCount, GroupchatKey,
        IterationCount, LoungeKey, NotebookKey, PassiveKey, Time,
    },
    role::Role,
};

// commands with no recipient are considered "system" commands and are used to talk directly to the
// host or the backend of a frontend
//
// the frontend server is expected to intercept certain commands if they wish to implement host controls

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    pub timestamp: Time,
    pub recipient: Option<ActorKey>,
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

    /////=<TARGETTED>=/////

    // notify a specific player of a death. this can be done in any way. it can be put into the news
    // channel display, a dedicated list, etc...
    // the server doesn't need to intercept this because hosts can directly view the event log and
    // view/modify actor states
    Death {
        true_name: String,
        death_message: String,
        role: Role,
        notebook_transferred: bool,
        ability_transferred: bool,
    },

    // display/announce kidnapping. can be handled similar to death.
    Kidnapping {
        target_id: ActorKey,
        duration: Time,
    },

    // announce a kidnap reveal (this will either leak the kidnapper or show no kidnapper meaning it
    // was anonymous)
    KidnapReveal {
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

    /////=<TARGETTED>=/////

    // display a player as an org member
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

    // map a lounge id to a channel id
    MapLounge {
        lounge_id: LoungeKey,
        channel_id: ChannelKey,
    },

    // map a gc id to a channel id
    MapGc {
        gc_id: GroupchatKey,
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

    /////=<TARGETTED>=/////

    // show the player the notebook's borrower status (dont show who is lending it, just that it is
    // being borrowed)
    NotebookBorrowingStatus {
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

    // update the view of an ability to reflect its current state
    UpdateAbilityView {
        ability_name: AbilityName,
        usages_remaining: ChargeCount,
        iterations_to_reset: IterationCount,
        ability_id: AbilityKey,
        owner_id: ActorKey,
    },

    // entirely hide an ability from a user
    RemoveAbility {
        ability_id: AbilityKey,
    },

    // tell the frontend to display autopsy messages for a specific user. the frontend server will do the
    // querying and filtering, and the clients will handle the display of that info.
    RevealAutopsyMessages {
        target_id: ActorKey,
        range: Time,
        redact_names: bool,
    },
    ////////////////////////////////////////////////
    // POLLS //
    ////////////////////////////////////////////////
}
