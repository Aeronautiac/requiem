/*
* lawliet
* a high performance deterministic headless engine written in rust for a multi-day death note social deduction game
*
* --- core engine ---
* the Engine owns: World (all game state in typed slotmaps), Config (dynamic runtime tuning),
* Jobs (min-heap priority queue of scheduled events), and a deferred command buffer.
* actions are validated in a non-mutating dry-run pass first, then executed if valid. sub-actions
* are invoked recursively and share the same command buffer across the entire action tree. pending
* jobs are flushed before each requested action to maintain temporal causality. time is a u128 of
* unix milliseconds. the engine panics on inconsistent state — it is designed to be rolled back
* by replaying the saved action log.
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
* polls carry optional accept/reject action payloads that fire on resolution. visibility scopes:
* Org, Channel, or AllPresent. VoterPolicy: Present (not dead/imprisoned/kidnapped, can see poll).
* PollPolicy: AlwaysInconclusive, Majority (>50%), or WinningVote (highest count; ties inconclusive).
* update_policy is checked on each vote; timeout_policy fires when the timer expires. vote weight:
* organizations vote 0; players vote 1, or more with a VoteAmplification passive.
*
* --- prosecution ---
* three-phase state machine: Custody -> Trial -> Voting.
* Custody: both sides signal ready or a timeout fires; non-autonomous requires host approval.
* Trial: two-sided subphases (Grace -> Presentation) then Debate. Grace starts immediately; the
* first message from either side triggers Presentation; Debate ends when both signal done or timeout.
* Voting: anonymous poll; guilty majority executes the defendant.
* ProsecutionDefense holds the defendant and an optional Lawyer with a private channel.
* the autonomous flag bypasses host approval for all phase transitions.
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
* CommandPayload carries a timestamp, optional recipient, and a Command variant. DeferredCommand
* adds blocking_modifiers — the command is withheld until the recipient has none of them active.
* command coverage: Death, Kidnapping, PseudocideRevival, ActorState, channel lifecycle (add
* message, map lounge/gc, delete, archive), notebook ops (map, write, borrow status), ability /
* passive views, bug events (new, message, archive, delete), and iteration progress. commands
* without a recipient are forwarded to the backend; recipient-targeted commands go to a specific
* player's client.
*
* --- yagami (external) ---
* hosts multiple lawliet instances and communicates via IPC. acts as persistence and routing layer.
* game state is never snapshotted — it is reconstructed from a saved action log. action buffers
* are flushed to postgres on timer, significant action, or full buffer. multithreaded process.
*
* --- frontend protocol ---
* frontends are dumb: they receive commands and errors and render accordingly. frontend servers
* handle routing. response data structs are used internally (tests, sub-actions, yagami). each
* frontend must support host controls and player game views.
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
mod test_helpers;
mod world;

pub use common::{
    AbilityKey, ActorKey, BugKey, ChannelKey, ChargePoolKey, GroupchatKey, ID, KidnappingKey,
    LoungeKey, NotebookKey, PassiveKey, PollKey, ProsecutionKey, Time,
};

// I've realized now that channels can likely be abstracted a bit further with overrides as a native
// object rather than being specific to world channels, but everything is basically done already, so
// theres no point in changing it at this point.

// TODO:
// - Finish non-autonomous prosecutions
// - Go through everything and implement frontend commands. Refine the command protocol.
// - Implement every ability and write tests for them
// - Integration tests
// - yagami
// - Add destroy actions for the different kinds of objects (actors will be the final destroyable objects. they may get very messy.)
// - Optimize by going through and caching what can be cached, adding indirection for very large
// enums, and using smallvec when possible

// TODO:
// - Test personal channels
//  * players should be allowed to change the loggable status of these channels
// - Test prosecution system
// - Test org channels
//  * orgs get their own private channel
//  * members are added to the channel when they join the org, and are removed when they leave
//  * their access to the channel should be determined by the same rules as contact channels (will
//  be evaluated in the same action)
//  * org membership should be cached within the player
// - Test world progression
//  * notebooks are progressed
//  * charge pools are progressed
//  * iteration counter is incremented
//  * bugs are ended
//  * ipp is ended
// - Add command outputs for strictly increasing contact channel ids used for ui display and abilities like tap in. The frontend
// can't reasonably expect players to enter slotmap keys. Sharing the ids will allow for things like
// tapping into group chats without being too easy as well.
// * the world holds the counter and maintains a map of ids to lounges/gcs
// * ensure that they are unmapped when the objects are destroyed

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
