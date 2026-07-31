/*
* lawliet
* a high performance deterministic headless engine written in rust for a multi-day death note social deduction game
*
* --- core engine ---
* the Engine owns: World (all game state in typed slotmaps), Config (dynamic runtime tuning),
* Jobs (min-heap priority queue of scheduled events).
* actions are validated in a non-mutating dry-run pass first, then executed if valid. sub-actions
* are invoked recursively and share the same command buffer across the entire action tree. pending
* jobs are flushed before each requested action to maintain temporal causality. time is a u128 of
* unix milliseconds. the engine panics on inconsistent state — it is designed to be rolled back
* by replaying the saved action log.
* a job holds an absolute timestamp and so cannot be paused. anything whose deadline may need to
* stop and start again holds a Timer instead — an object in the world, keyed like any other, which
* owns a job and the arithmetic for banking what a run served. UpdateTimers stops and starts every
* timer that opted into being stopped, in one sweep that knows nothing about what any of them are
* for.
*
* --- actors ---
* players and organizations are both actors. actor structs carry IndexSets of ability, passive,
* and notebook keys — these are caches for performance and utility; true ownership is tracked
* within each respective struct and must be kept in sync with the actor cache.
* players additionally cache lounge, groupchat, and bug keys (bugs = wiretaps targeting them).
* players also carry: role, true name, eye count, and per-channel world channel overrides (keyed
* by WorldChannelName; sources are Role, Manual host id, or PressConference; ties at the same
* priority level are resolved via Positive (OR) or Negative (AND)).
* organizations carry: an optional leader, a members map with founding-member metadata,
* a blacklist, and per-ability policy rules (RequireLeader, RequireVote).
* both actor types accumulate states (Dead, Incarcerated, IPP, Kidnapped, Custody) and modifiers
* (NoPresence, NoContact, WriteImmunity, DisablePassiveLinks, etc.) as enumflag2 bitfields, keyed
* by source so that overlapping additions from different sources are removed independently.
* actors can be linked bidirectionally — Life links chain death/revive events; Passive links cause
* an actor to inherit another actor's passives (severed on death unless disabled).
*
* --- config ---
* config is dynamic; changing it is an action and takes effect immediately. RoleConfig maps each
* role to its default abilities, passives, notebooks, actor links, and world channel overrides.
* AbilityConfig maps ability identifiers (type + optional variant) to default charge pool links
* and presence requirements. StateModifierMap and WorldConfig hold global defaults.
*
* --- abilities and passives ---
* abilities have an OwnershipStruct (owner, volatile, transferrable) and a set of AbilityPoolLinks
* to ChargePool objects. get_usage_limit() returns the minimum available uses across Limit-type
* pools, falling back to max from Pool-type pools. volatile abilities are destroyed on role change;
* transferrable abilities survive death. passives mirror the same ownership model. passive types:
* Wanted (silent prosecution immunity), VoteAmplification, VolatileEyes, ContactLogs (Full/Even/Odd),
* OwnedNotebookBlock, CustodyBugReceiver. actor_get_effective_passive traverses actor links
* recursively to collect inherited passives, respecting the DisablePassiveLinks modifier.
*
* --- notebooks ---
* notebooks are real or fake (fake writes cannot kill) and volatile or persistent. ownership chain:
* original_owner -> owner -> borrowed (temporary holder). dormant_true_owner supports pseudocide
* revival mechanics. per-actor success/failure counts reset at each iteration boundary.
* lend() and awaken_dormant_owner() manage the full borrowing lifecycle.
*
* --- charge pools ---
* both PoolLinkTypes subtract weight charges from every linked ability on use. the difference is
* in the failure condition: Limit fails if any linked ability cannot afford the cost; Pool fails
* only if none of the linked abilities can afford the cost. charges decay per iteration via
* base_reset_time. on_use() deducts weight charges; add_charges() replenishes. on_link/on_unlink
* track reference counts — unlink returning true signals the pool is safe to destroy.
*
* --- polls ---
* a poll is a list of options, each with an optional action payload that fires if it wins. votes
* name an option by index, and the list is fixed for the poll's life — an option whose payload
* stops validating cancels the whole poll rather than being dropped out from under the votes.
* PollPolicy: AlwaysInconclusive, Majority (an option holding >50% of the possible weight), or
* MostVoted (the heaviest option; ties inconclusive, as is a poll nobody voted in). update_policy
* is checked on each vote; timeout_policy fires when the timer expires. vote weight: organizations
* vote 0; players vote 1, or more with a VoteAmplification passive — ignore_amplification clamps
* every voter back to one. VoterPolicy: Present (not dead/imprisoned/kidnapped, in the poll's
* scope).
*
* a poll owns no viewport. it hangs off a parent — Org, Channel, or World — and is addressed to
* that parent's viewport, so it inherits every reason its audience can shrink and dies when its
* parent does. the parent answers two questions that must not be confused: its MEMBERSHIP is who
* the poll is put to, its VIEWPORT is who can reach the ballot right now.
*
* that gap is why counting a vote and entering one are separate questions. counts() is the tally
* rule and reads the voter policy; can_enter() is counts() plus can_view(), and is what
* AddVote/RemoveVote check. a blackout opens the gap — a world poll goes off the air, so nobody
* may touch their vote, while every vote already cast still counts and the poll can still resolve
* on what it holds.
*
* --- prosecution ---
* three-phase state machine: Custody -> Trial -> Voting.
* Custody: both sides signal ready or a timeout fires.
* Trial: two-sided subphases (Grace -> Presentation) then Debate. Grace starts immediately; the
* first message from the side holding the floor triggers Presentation; Debate ends when both signal
* done or timeout.
* Voting: anonymous poll; guilty majority executes the defendant. CloseProsecution carries the
* verdict, which for an acquittal is the only trace it left.
* ProsecutionDefense holds the defendant and an optional Lawyer with a private channel.
* the autonomous flag, when false, holds the two MAJOR boundaries (Custody -> Trial, Debate ->
* Voting) until an admin AdvanceProsecution arrives; subphase movement inside the trial never waits.
* NoPresence on the prosecutor or defendant terminates prosecution immediately.
* a custody bug is auto-created to wiretap the defendant for the duration.
*
* --- kidnapping ---
* wraps a private channel with kidnapping metadata. type: Anonymous or Public(ActorDisplay).
* applying a kidnapping sets the Kidnapped state on the victim, which carries whatever modifiers
* are associated with that state in config.
*
* --- channels, lounges, groupchats, bugs ---
* Channel is the primitive: a members map with Send/View permission bitflags per actor and a
* loggable flag for ability queries (e.g. autopsy). no message storage — yagami handles that.
* Lounge wraps a channel for two-actor contact; Fake lounges expose the true creator's identity to
* a tapper without the creator's knowledge. Groupchat wraps a channel with an optional owner;
* owner leaving sets owner to None. Bug is a wiretap on a target actor, sourced from an ability or
* a custody event; expired bugs are retained in memory for persistent history access.
*
* --- commands ---
* CommandPayload carries a timestamp, a recipient, and a Command variant. every command is
* addressed: System (the host, and by extension the admin's omniscient mirror), Actor (one
* player or org), or Viewport.
*
* a viewport is an opaque engine-allocated identity that commands are addressed to. objects that
* gate visibility — channels, bugs, passives, and the world itself — each own one, and
* each writes its viewport from its OWN visibility rule; the viewport is never consulted to
* answer "can this actor see X?", only "who do I send this to?". gaining access delivers
* everything previously addressed to that viewport, in order, which is what carries history to a
* late arrival. losing access only stops further delivery: client state is monotonic and nothing
* is ever retracted, so deletion is expressed as archival.
*
* the world owns two viewports rather than one. the events viewport (every player without
* Modifier::NoPresence) is what makes world events presence-gated, and is what a blackout empties.
* the data viewport holds every player from creation and is never revoked by anything, because
* existence and the clock must reach a player who cannot see anything else. together they replaced
* both the BasePlayer catch-up stream and the deferred command queue: an absent player is simply
* not a member of the events one, and re-entry backfills the backlog in order.
*
* command coverage: Death, Kidnapping, PseudocideRevival, channel lifecycle (add message, map
* lounge/gc, archive), notebook ops (map, write, borrow status), ability / passive views, bug
* events (new, message, archive), and iteration progress.
*
* --- yagami (external) ---
* yagami is the central server. it handles the platform/auth as well as hosting multiple lawliet
* instances and communicates via IPC. it acts as persistence and routing layer to different clients.
* game state is never snapshotted — it is reconstructed by replaying a durable action log, which is
* the source of truth. persistence is write-ahead: an action is appended to the log (idempotent by
* sequence) and made durable BEFORE its result is acked/broadcast, so a crash can lose only an
* un-acked (therefore retryable) action, never confirmed state. multithreaded process.
* in the case that a frontend requires another server layer, the server may present itself as an
* authoritative client to yagami and relay to its sub-clients.
*
* --- frontend protocol ---
* frontends are dumb: they receive commands and errors and render accordingly.
* response data structs are used in cases where a direct response to an action is necessary
* (creating players and receiving their ids for instance). commands are used for everything else.
* each frontend must support host controls and player game views.
*/

mod ability;
mod action;
mod actor;
mod bug;
mod channel;
mod chargepool;
mod command;
mod common;
mod config;
pub mod engine;
mod groupchat;
mod helpers;
mod incarceration;
mod kidnapping;
mod lounge;
mod notebook;
mod ownership;
mod passive;
mod poll;
mod prosecution;
#[cfg(test)]
mod test_helpers;
mod timer;
mod viewport;
mod world;

pub use common::{
    AbilityKey, ActorKey, BugKey, ChannelKey, ChargePoolKey, GroupchatKey, ID, KidnappingKey,
    LoungeKey, NotebookKey, PassiveKey, PollKey, ProsecutionKey, Time,
};

// I've realized now that channels can likely be abstracted a bit further with overrides as a native
// object rather than being specific to world channels, but everything is basically done already, so
// theres no point in changing it at this point.

// TODO:
// - Channel rework
// - Press conferences
// - A poll's deadline never reaches the client. UpdatePoll carries no remaining time, so a poll
//   whose timer is paused looks exactly like one that is simply taking a while. Nothing renders a
//   countdown yet; whatever does has to be told about the pause along with the time.
// - Rulesets & live config editing
// - Add destroy actions for the different kinds of objects (actors will be the final destroyable objects. they may get very messy.)
// - Optimize by going through and caching what can be cached, adding indirection for very large
// enums, and using smallvec when possible

#[cfg(test)]
mod tests {
    use crate::{
        actor::state::State,
        config::role::Role,
        engine::Engine,
        helpers::{actor_get_effective_passive, get_ability, get_actor, get_passive},
        passive::{ContactLogType, PassiveType},
        test_helpers::*,
    };

    // Regression: PurgeVolatiles formerly removed volatile resources from world maps but not from
    // the actor's own ID sets. On a second role change, PurgeVolatiles would iterate stale IDs and
    // panic. Verified by cycling through a role with volatile resources twice.
    #[test]
    fn repeated_role_change_purges_stale_ids() {
        let mut eng = Engine::new();
        init_engine(&mut eng);

        let p1 = add_player(&mut eng, 0, Role::NewsAnchor, "p1"); // gains ability + passive

        give_role(&mut eng, 0, p1, Role::Civilian); // purges NewsAnchor volatiles
        give_role(&mut eng, 0, p1, Role::NewsAnchor); // would panic before the fix

        let actor = get_actor(&eng, p1).unwrap();

        // all IDs in actor.abilities must resolve in the world
        for &id in &actor.abilities {
            assert!(
                get_ability(&eng, id).is_ok(),
                "stale ability id {id:?} in actor cache"
            );
        }

        // all IDs in actor.passives must resolve in the world
        for &id in &actor.passives {
            assert!(
                get_passive(&eng, id).is_ok(),
                "stale passive id {id:?} in actor cache"
            );
        }
    }

    // Link behaviour:
    // Links are not severed if the death was caused by a link
    // If the death was not caused by a link, they are typically severed, though this can be
    // disabled as well
    #[test]
    fn l_watari_links() {
        let mut eng = Engine::new();
        init_engine(&mut eng);

        let w_id_1 = add_player(&mut eng, 0, Role::Watari, "John Candlewick");
        let l_id = add_player(&mut eng, 3, Role::L, "John Pork");
        let w_id_2 = add_player(&mut eng, 5, Role::Watari, "Oima Haumzaundwich");

        assert!(
            actor_get_effective_passive(&eng, l_id, |passive_type| {
                matches!(passive_type, PassiveType::ContactLogs(ContactLogType::Full))
            })
            .is_some()
        );

        // link to this one should be severed now
        quick_kill(&mut eng, 5, false, true, false, w_id_1);

        // L should still be linked to watari 1
        assert!(
            actor_get_effective_passive(&eng, l_id, |passive_type| {
                matches!(passive_type, PassiveType::ContactLogs(ContactLogType::Full))
            })
            .is_some()
        );

        // this one should only kill watari 2 and L
        // links should remain intact
        quick_kill(&mut eng, 6, true, true, false, l_id);

        let watari1 = get_actor(&eng, w_id_1).unwrap();
        let watari2 = get_actor(&eng, w_id_2).unwrap();
        assert!(watari1.has_state(State::Dead) && watari2.has_state(State::Dead));

        // this one should only revive L
        quick_revive(&mut eng, 6, true, l_id);

        // the passive link to watari 2 should still be intact although disabled due to the passive
        // link restriction on watari 2
        assert!(
            actor_get_effective_passive(&eng, l_id, |passive_type| {
                matches!(passive_type, PassiveType::ContactLogs(ContactLogType::Full))
            })
            .is_none()
        );

        // links were ignored, so only L should have been revived
        let watari1 = get_actor(&eng, w_id_1).unwrap();
        let watari2 = get_actor(&eng, w_id_2).unwrap();
        assert!(watari1.has_state(State::Dead) && watari2.has_state(State::Dead));

        // kill L again, do not sever links, and do not allow chaining
        quick_kill(&mut eng, 6, false, false, false, l_id);

        // this should revive watari 2 along with L
        quick_revive(&mut eng, 6, false, l_id);

        // the passive link should be enabled again because there is no passive link restriction
        assert!(
            actor_get_effective_passive(&eng, l_id, |passive_type| {
                matches!(passive_type, PassiveType::ContactLogs(ContactLogType::Full))
            })
            .is_some()
        );

        // only watari 2 and L should be revived as watari 1 died alone
        let watari1 = get_actor(&eng, w_id_1).unwrap();
        let watari2 = get_actor(&eng, w_id_2).unwrap();
        assert!(watari1.has_state(State::Dead) && !watari2.has_state(State::Dead));
    }
}
