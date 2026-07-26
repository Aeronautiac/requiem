/*
* Authoritative Action
* Loop through every bug, clear visibilities, evaluate new visibilities based on current
* conditions
*/

// should be called when:
// - ability ownership changes
// - a bug is created
// - an actor's state changes
//
// this can be optimized later, but just calling it in these cases massively simplifies things
// without having to put it into the update action

use indexmap::IndexSet;
use smallvec::{SmallVec, smallvec};

use crate::{
    ActorKey,
    action::{ActionInterface, ActionResponse},
    actor::{ActorType, modifier::Modifier},
    bug::BugSource,
    common::{BugKey, ViewportKey},
    helpers::{actor_get_effective_passive, get_ability, get_actor, sync_viewport},
    passive::PassiveType,
};

use crate::action::ActionActor;
pub use crate::action::{UpdateBugVisibilities, UpdateBugVisibilitiesResponse};

impl ActionInterface for UpdateBugVisibilities {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let mut custody_viewers: SmallVec<[ActorKey; 8]> = smallvec![];
        for (key, _) in eng.world.actors.iter().filter(|(_, act)| {
            matches!(act.actor_type, ActorType::Player(_))
                && !act.has_modifier(Modifier::NoPresence)
        }) {
            if actor_get_effective_passive(eng, key, |passive| {
                matches!(passive, PassiveType::CustodyBugReceiver)
            })
            .is_some()
            {
                custody_viewers.push(key)
            }
        }

        // Compute every bug's viewers first, then apply. Applying inline is not an option:
        // sync_viewport needs &mut Engine, and the iteration is borrowing the bug map.
        let bugs: SmallVec<[BugKey; 16]> = eng.world.bugs.keys().collect();
        let mut resolved: SmallVec<[(ViewportKey, IndexSet<ActorKey>); 16]> = smallvec![];
        for key in bugs {
            let bug = eng.world.get_bug(key).expect("just enumerated");
            let viewport = bug.viewport;
            let mut viewers = IndexSet::new();
            match &bug.source {
                BugSource::Ability(ability_id) => {
                    let ability = get_ability(eng, *ability_id)?;
                    if let Some(owner) = ability.ownership_struct.owner
                        && !get_actor(eng, owner)?.has_modifier(Modifier::NoPresence)
                    {
                        viewers.insert(owner);
                    }
                }
                BugSource::Custody => viewers.extend(custody_viewers.iter().copied()),
            }
            resolved.push((viewport, viewers));
        }

        // This whole action is a recompute-from-scratch, and it runs on every ability-ownership
        // change, actor state change and bug creation. It used to emit a ClearBugVisibily for
        // every bug in the world and then re-add whoever still qualified, whether or not
        // anything had changed. The diff turns that back into the handful of commands that
        // represent real access changes.
        for (viewport, viewers) in resolved {
            sync_viewport(eng, ctx, viewport, viewers, mutate);
        }

        Ok(ActionResponse::UpdateBugVisibilities(
            UpdateBugVisibilitiesResponse {},
        ))
    }
}
