/*
* SYSTEM ACTION
* Check all active prosecutions for forced termination conditions.
* Called from Update so it runs once per action rather than from individual actions.
*
* Poll resolution is not handled here — when the prosecution poll concludes it calls a
* dedicated action to apply the verdict and terminate the prosecution.
*
* All phases:
*   if the source is an ability, and the ability doesn't exist → TerminateProsecution
*
* Custody or Trial phase:
*   if prosecutor or defendant has NoPresence → TerminateProsecution
*   if the source is an ability, the ability is owned by an organization, and the prosecutor is no
*   longer part of that organization → TerminateProsecution
*
* Voting phase:
*   if defendant is dead → TerminateProsecution
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, TerminateProsecution,
    },
    actor::{ActorType, modifier::Modifier, state::State},
    common::{ProsecutionKey, Version},
    engine::Engine,
    helpers::get_actor,
    prosecution::{ProsecutionPhase, ProsecutionSource},
};

pub use crate::action::{CullProsecutions, CullProsecutionsResponse};

impl ActionInterface for CullProsecutions {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let to_terminate: Vec<ProsecutionKey> = eng
            .world
            .prosecutions
            .iter()
            .filter_map(|(key, prosecution)| {
                let prosecutor = get_actor(eng, prosecution.prosecutor)
                    .expect("prosecutor must be a valid actor");
                let defendant = get_actor(eng, prosecution.defense.defendant)
                    .expect("defendant must be a valid actor");

                let should_terminate = 'check: {
                    // All phases: source ability destroyed
                    if let ProsecutionSource::Ability(ab) = prosecution.source {
                        if eng.world.get_ability(ab).is_none() {
                            break 'check true;
                        }
                    }

                    match &prosecution.phase {
                        ProsecutionPhase::Custody { .. } | ProsecutionPhase::Trial { .. } => {
                            if prosecutor.has_modifier(Modifier::NoPresence)
                                || defendant.has_modifier(Modifier::NoPresence)
                            {
                                break 'check true;
                            }

                            // Source ability owned by org but prosecutor left that org
                            if let ProsecutionSource::Ability(ab) = prosecution.source {
                                if let Some(ability) = eng.world.get_ability(ab) {
                                    if let Some(owner_id) = ability.ownership_struct.owner {
                                        let owner = get_actor(eng, owner_id)
                                            .expect("ability owner must be a valid actor");
                                        if let ActorType::Org(org) = &owner.actor_type {
                                            if !org.members.contains_key(&prosecution.prosecutor) {
                                                break 'check true;
                                            }
                                        }
                                    }
                                }
                            }

                            false
                        }
                        ProsecutionPhase::Voting { .. } => defendant.has_state(State::Dead),
                    }
                };

                should_terminate.then_some(key)
            })
            .collect();

        for prosecution_id in to_terminate {
            Action::TerminateProsecution(TerminateProsecution { prosecution_id }).handle(
                eng,
                ctx,
                &ActionActor::System,
                version,
                mutate,
            )?;
        }

        Ok(ActionResponse::CullProsecutions(
            CullProsecutionsResponse {},
        ))
    }
}
