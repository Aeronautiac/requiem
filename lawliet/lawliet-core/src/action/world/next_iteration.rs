use lawliet_types::{
    action::{
        Action, ActionActor, ActionError, ActionRequest, ActionResponse, ArchiveBug, NextIteration,
        ReturnBorrowedNotebooks,
    },
    actor::State,
    bug::BugSource,
    command::Command,
    common::{AbilityKey, ActorKey, BugKey, ChargePoolKey},
    world::WorldPhase,
};
use smallvec::SmallVec;

use crate::{
    action::ActionInterface,
    helpers::{cmd_ability_view, cmd_world_data, get_ability},
};

impl ActionInterface for NextIteration {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        version: lawliet_types::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        if eng.world.phase != WorldPhase::Running {
            return Err(ActionError::GameNotStarted);
        }

        if mutate {
            for (_, notebook) in eng.world.notebooks.iter_mut() {
                notebook.iteration_reset();
            }

            // Only a pool counting down can change on the tick: its countdown moves, and at zero its
            // charges refill. Every ability drawing on one has a new view to send.
            let mut ticked: SmallVec<[ChargePoolKey; 16]> = SmallVec::new();
            for (id, pool) in eng.world.charge_pools.iter_mut() {
                if pool.iterations_to_reset != 0 {
                    ticked.push(id);
                }
                pool.on_iteration();
            }
            let mut stale: SmallVec<[(ActorKey, AbilityKey); 16]> = SmallVec::new();
            for (owner_id, owner) in eng.world.actors.iter() {
                for &ability_id in owner.abilities.iter() {
                    let ability = get_ability(eng, ability_id)?;
                    if ability.pool_links.iter().any(|l| ticked.contains(&l.link.link_dest)) {
                        stale.push((owner_id, ability_id));
                    }
                }
            }
            for (owner_id, ability_id) in stale {
                cmd_ability_view(eng, ctx, owner_id, ability_id)?;
            }

            // Iteration-scoped states: they last until the boundary and no further, so there is
            // nothing per-actor to count down.
            for (_, actor) in eng.world.actors.iter_mut() {
                actor.remove_state(State::Ipp);
                actor.remove_state(State::UnderTheRadar);
            }

            eng.world.curr_iteration += 1;
        }

        // Planted bugs last a day. A custody wiretap is not planted and does not expire with one —
        // it belongs to the custody, and lasts exactly as long as the prosecution keeps its
        // defendant there. SetCustody is the only thing that ends it.
        let keys: SmallVec<[BugKey; 8]> = eng
            .world
            .bugs
            .iter()
            .filter_map(|(id, bug)| matches!(bug.source, BugSource::Ability(_)).then_some(id))
            .collect();
        for id in keys {
            Action::ArchiveBug(ArchiveBug { bug_id: id })
                .handle(eng, ctx, actor, version, mutate)?;
        }

        Action::ReturnBorrowedNotebooks(ReturnBorrowedNotebooks {}).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;

        // Re-arm, or leave the clock alone if the host owns it.
        //
        // The pending advance is cancelled first, which is what makes an early manual turn behave:
        // the new day gets a full duration instead of being cut short by a timer belonging to the
        // day before. A natural turn cancels a job that has already fired, which is a no-op.
        if mutate {
            if let Some(job) = eng.world.iteration_job.take() {
                eng.jobs.cancel_id(job);
            }
            if eng.config.defaults.iterations_autonomous {
                let at = eng.time + eng.config.defaults.iteration_duration;
                let job = eng.jobs.push(ActionRequest {
                    actor: ActionActor::System,
                    timestamp: at,
                    payload: Action::NextIteration(NextIteration {}),
                });
                eng.world.iteration_job = Some(job);
            }
        }

        // World DATA: the clock is not an announcement. A prisoner and a blacked-out world both
        // still need to know what day it is, or nothing else they can see makes sense.
        cmd_world_data(
            eng,
            ctx,
            Command::NewIteration {
                iteration: eng.world.curr_iteration,
            },
        );

        Ok(ActionResponse::NextIteration(
            lawliet_types::action::NextIterationResponse {},
        ))
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, UnderTheRadar},
        action::CreateAndGiveAbility,
        command::Command,
    };

    use crate::{
        action::{Action, ActionActor, ActionContext, ActionRequest, NextIteration},
        common::{AbilityKey, ActorKey},
        config::role::Role,
        engine::Engine,
        helpers::get_ability,
        test_helpers::{add_player, init_engine, quick_ability, use_ability},
    };

    fn world(eng: &mut Engine) -> (ActorKey, AbilityKey) {
        init_engine(eng);
        let user = add_player(eng, 0, Role::Civilian, "user");
        let ability = quick_ability(
            eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::UnderTheRadar,
                variant: 0,
                actor_id: user,
                volatile: false,
                transferrable: false,
            },
        );
        (user, ability)
    }

    fn turn_day(eng: &mut Engine, time: crate::Time) -> ActionContext {
        eng.execute(
            ActionRequest {
                actor: ActionActor::System,
                timestamp: time,
                payload: Action::NextIteration(NextIteration {}),
            },
            Engine::version(),
        )
        .unwrap()
        .1
    }

    fn views_of(ctx: &ActionContext, ability: AbilityKey) -> Vec<&Command> {
        ctx.commands
            .iter()
            .map(|p| &p.cmd)
            .filter(|cmd| {
                matches!(cmd, Command::UpdateAbilityView { ability_id, .. } if *ability_id == ability)
            })
            .collect()
    }

    // A use arms the pool's countdown; the day turning moves it, so the owner is told where the
    // ability stands now rather than left looking at the counts from the use.
    #[test]
    fn the_day_turning_resends_a_used_abilitys_view() {
        let mut eng = Engine::new();
        let (user, ability) = world(&mut eng);
        use_ability(&mut eng, 1, user, ability, AbilityBehaviour::UnderTheRadar(UnderTheRadar {}))
            .unwrap();

        let ctx = turn_day(&mut eng, 2);

        let views = views_of(&ctx, ability);
        assert_eq!(views.len(), 1);
        let Command::UpdateAbilityView {
            success_usages_remaining,
            failure_usages_remaining,
            iterations_to_reset,
            base_reset,
            ..
        } = views[0]
        else {
            unreachable!()
        };
        let now = get_ability(&eng, ability).unwrap().get_ability_view_counts(&eng);
        assert_eq!(
            (*success_usages_remaining, *failure_usages_remaining, *iterations_to_reset, *base_reset),
            now
        );
    }

    // A pool nobody has drawn on is not counting down, so the tick changes nothing about it.
    #[test]
    fn an_unused_ability_is_not_resent() {
        let mut eng = Engine::new();
        let (_, ability) = world(&mut eng);

        let ctx = turn_day(&mut eng, 1);

        assert!(views_of(&ctx, ability).is_empty());
    }
}
