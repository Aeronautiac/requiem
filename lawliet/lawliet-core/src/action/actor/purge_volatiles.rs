/*
* SYSTEM ACTION
* Destroy all volatile resources associated with an actor
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, DestroyAbility, DestroyNotebook, DestroyPassive,
    },
    common::{AbilityKey, NotebookKey, PassiveKey},
    helpers::{get_ability, get_actor, get_notebook, get_passive},
};

pub use crate::action::{PurgeVolatiles, PurgeVolatilesResponse};

impl ActionInterface for PurgeVolatiles {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let target_actor = get_actor(eng, self.actor_id)?;
        let mut remove_abilities: Vec<AbilityKey> = vec![];
        let mut remove_passives: Vec<PassiveKey> = vec![];
        let mut remove_notebooks: Vec<NotebookKey> = vec![];
        for id in target_actor.abilities.iter() {
            let ability = get_ability(eng, *id).expect("actor references non-existent ability: engine invariant violated");
            if ability.ownership_struct.volatile {
                remove_abilities.push(*id);
            }
        }
        for id in target_actor.passives.iter() {
            let passive = get_passive(eng, *id).expect("actor references non-existent passive: engine invariant violated");
            if passive.ownership_struct.volatile {
                remove_passives.push(*id);
            }
        }
        for id in target_actor.notebooks.iter() {
            let notebook = get_notebook(eng, *id).expect("actor references non-existent notebook: engine invariant violated");
            if notebook.volatile {
                remove_notebooks.push(*id);
            }
        }

        for id in remove_abilities {
            Action::DestroyAbility(DestroyAbility { ability_id: id })
                .handle(eng, ctx, actor, version, mutate)?;
        }
        for id in remove_passives {
            Action::DestroyPassive(DestroyPassive { passive_id: id })
                .handle(eng, ctx, actor, version, mutate)?;
        }
        for id in remove_notebooks {
            Action::DestroyNotebook(DestroyNotebook { notebook_id: id })
                .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::PurgeVolatiles(PurgeVolatilesResponse {}))
    }
}
