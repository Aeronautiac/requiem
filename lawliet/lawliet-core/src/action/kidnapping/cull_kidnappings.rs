/*
* SYSTEM ACTION
* Force-release all kidnappings whose source ability was just destroyed.
* Called exclusively from DestroyAbility with the destroyed ability's key.
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, ReleaseKidnapping,
    },
    common::{AbilityKey, KidnappingKey, Version},
    engine::Engine,
    kidnapping::KidnappingSource,
};

pub use crate::action::{CullKidnappings, CullKidnappingsResponse};

impl ActionInterface for CullKidnappings {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let to_release: Vec<KidnappingKey> = eng
            .world
            .kidnappings
            .iter()
            .filter_map(|(key, k)| {
                matches!(k.source, KidnappingSource::Ability(ab) if ab == self.ability_id)
                    .then_some(key)
            })
            .collect();

        for kidnapping_id in to_release {
            Action::ReleaseKidnapping(ReleaseKidnapping {
                kidnapping_id,
                forced: true,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::CullKidnappings(CullKidnappingsResponse {}))
    }
}
