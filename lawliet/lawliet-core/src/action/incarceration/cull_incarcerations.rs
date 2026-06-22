/*
* SYSTEM ACTION
* Force-release all incarcerations whose source ability was just destroyed.
* Called exclusively from DestroyAbility with the destroyed ability's key.
*/

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionResponse, ReleaseIncarceration,
    },
    common::{AbilityKey, IncarcerationKey, Version},
    engine::Engine,
    incarceration::IncarcerationSource,
};

pub use crate::action::{CullIncarceratationsResponse, CullIncarcerations};

impl ActionInterface for CullIncarcerations {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let to_release: Vec<IncarcerationKey> = eng
            .world
            .incarcerations
            .iter()
            .filter_map(|(key, i)| {
                matches!(i.source, IncarcerationSource::Ability(ab) if ab == self.ability_id)
                    .then_some(key)
            })
            .collect();

        for incarceration_id in to_release {
            Action::ReleaseIncarceration(ReleaseIncarceration {
                incarceration_id,
                forced: true,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::CullIncarcerations(
            CullIncarceratationsResponse {},
        ))
    }
}
