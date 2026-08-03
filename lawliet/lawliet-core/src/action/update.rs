/*
* SYSTEM ACTION
* Keep game state up to date for anything that is fairly isolated but dependent
* on everything else in game and may in of itself influence game state
*/

pub use crate::action::{
    Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
    UpdateActorStatuses, UpdateChannels, UpdateKidnappings, UpdateOrgEffectiveMembers, UpdatePolls,
    UpdatePrisonChannel, UpdateProsecutions, UpdateTimers, UpdateWorldViewports,
};

pub use crate::action::{Update, UpdateResponse};

impl ActionInterface for Update {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Access, recomputed rather than tracked. Who can reach the world viewports and what
        // anyone may do in a world channel both depend on state that moves all over the engine —
        // presence, roles, overrides, blackout — and both are cheap and silent when the answer
        // has not changed, so they are answered once here instead of at every site that could
        // have moved an input.
        //
        // Both run twice, before the subsystems and after, because the subsystems read access and
        // then move it. A poll concluding on the action that started a blackout has to be
        // announced to the audience the blackout leaves behind rather than the one it took away —
        // and the payload that concludes it kills people, which moves that audience again. Neither
        // pass can be dropped in favour of ordering the two against each other. Repeating a sweep
        // that says nothing when nothing changed is what makes that affordable.
        //
        // Channels are half of access, not a separate concern: a channel- or org-parented poll is
        // read through the channel's membership viewport, and this is the sweep that writes it.
        Action::UpdateWorldViewports(UpdateWorldViewports {})
            .handle(eng, ctx, actor, version, mutate)?;
        Action::UpdateChannels(UpdateChannels {}).handle(eng, ctx, actor, version, mutate)?;

        Action::UpdatePolls(UpdatePolls {}).handle(eng, ctx, actor, version, mutate)?;
        Action::UpdateProsecutions(UpdateProsecutions {})
            .handle(eng, ctx, actor, version, mutate)?;
        // After the channel sweep above, which is what decides who is taking part in an org — and
        // that is what this reads to work out who is holding somebody.
        Action::UpdateKidnappings(UpdateKidnappings {}).handle(eng, ctx, actor, version, mutate)?;
        Action::UpdatePrisonChannel(UpdatePrisonChannel {})
            .handle(eng, ctx, actor, version, mutate)?;

        Action::UpdateWorldViewports(UpdateWorldViewports {})
            .handle(eng, ctx, actor, version, mutate)?;
        Action::UpdateChannels(UpdateChannels {}).handle(eng, ctx, actor, version, mutate)?;

        // After the final viewport sweep, so a newly-created player is already on the world-data
        // viewport their status broadcasts to. Reads the settled states, bugs and blackout flag.
        Action::UpdateActorStatuses(UpdateActorStatuses {})
            .handle(eng, ctx, actor, version, mutate)?;

        // Reads the same settled presence the statuses do: who counts toward an org's ability
        // member requirements is exactly who is present.
        Action::UpdateOrgEffectiveMembers(UpdateOrgEffectiveMembers {})
            .handle(eng, ctx, actor, version, mutate)?;

        // Last, because a timer's gate is a viewport and every one of them has just settled.
        Action::UpdateTimers(UpdateTimers {}).handle(eng, ctx, actor, version, mutate)?;

        Ok(ActionResponse::Update(UpdateResponse {}))
    }
}
