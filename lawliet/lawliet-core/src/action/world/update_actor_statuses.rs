/*
* SYSTEM ACTION
* Broadcast every player's public Status on the world-data viewport.
*
* One place recomputes what the world may see of each actor — their states, whether they are
* bugged, whether the lights are on — instead of every site that could move one of those inputs.
* cmd_actor_status emits only on a genuine change (diffed against the actor's last_status), so
* sweeping every player on every Update costs nothing and says nothing when nothing has moved.
*/

use smallvec::SmallVec;

use crate::{
    action::{ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult},
    actor::ActorType,
    common::{ActorKey, Version},
    engine::Engine,
    helpers::cmd_actor_status,
};

pub use crate::action::{UpdateActorStatuses, UpdateActorStatusesResponse};

impl ActionInterface for UpdateActorStatuses {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        _version: Version,
        _mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let players: SmallVec<[ActorKey; 32]> = eng
            .world
            .actors
            .iter()
            .filter_map(|(id, a)| matches!(a.actor_type, ActorType::Player(_)).then_some(id))
            .collect();

        for id in players {
            cmd_actor_status(eng, ctx, id);
        }

        Ok(ActionResponse::UpdateActorStatuses(
            UpdateActorStatusesResponse {},
        ))
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::command::{Command, CommandRecipient};

    use crate::{
        action::{
            Action, ActionActor, ActionRequest, ActionResponse, ArchiveBug, CreateAndGiveAbility,
            CreateBug,
        },
        actor::state::{State, Status, Statuses},
        bug::BugSource,
        common::{ActorKey, BugKey, Time},
        config::{ability::AbilityName, role::Role},
        engine::Engine,
        helpers::get_actor,
        test_helpers::{add_player, add_state, init_engine, quick_ability, quick_kill, set_blackout},
    };

    fn engine_with_player() -> (Engine, ActorKey) {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let p = add_player(&mut eng, 0, Role::Civilian, "p");
        (eng, p)
    }

    // The last projection stored for the actor — equal to what was broadcast.
    fn status(eng: &Engine, id: ActorKey) -> Statuses {
        get_actor(eng, id).unwrap().last_status
    }

    fn plant_bug(eng: &mut Engine, time: Time, target: ActorKey, source: BugSource) -> BugKey {
        let (resp, _) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: time,
                payload: Action::CreateBug(CreateBug {
                    target_id: target,
                    source,
                }),
            })
            .unwrap();
        let ActionResponse::CreateBug(r) = resp else {
            unreachable!()
        };
        r.id
    }

    #[test]
    fn a_state_is_projected_on_the_world_data_viewport() {
        let (mut eng, p) = engine_with_player();
        let (_, ctx) = add_state(&mut eng, 1, p, State::Incarcerated);

        let (recipient, projected) = ctx
            .commands
            .iter()
            .find_map(|c| match &c.cmd {
                Command::ActorStatus { actor_id, status } if *actor_id == p => {
                    Some((c.recipient.clone(), *status))
                }
                _ => None,
            })
            .expect("a status is broadcast");

        assert_eq!(recipient, CommandRecipient::Viewport(eng.world.data_viewport));
        assert!(projected.contains(Status::Incarcerated));
        assert!(!projected.contains(Status::Missing));
        assert!(!projected.contains(Status::Dead));
    }

    // UnderTheRadar has no public flag and is not a presence loss, so it projects nothing at all —
    // being unseen is the whole point of it.
    #[test]
    fn under_the_radar_is_never_projected() {
        let (mut eng, p) = engine_with_player();
        let (_, ctx) = add_state(&mut eng, 1, p, State::UnderTheRadar);

        assert!(!ctx.commands.iter().any(|c| matches!(
            &c.cmd,
            Command::ActorStatus { actor_id, .. } if *actor_id == p
        )));
        assert_eq!(status(&eng, p), Statuses::empty());
    }

    #[test]
    fn an_explicit_bug_sets_and_archiving_clears_bugged() {
        let (mut eng, p) = engine_with_player();

        // CreateBug resolves an Ability source, so the bug must cite a real ability. Any resolvable
        // one will do — the projection only cares that the source isn't Custody.
        let ability = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                actor_id: p,
                ability_name: AbilityName::Gun,
                variant: 0,
                transferrable: false,
                volatile: false,
            },
        );
        let bug = plant_bug(&mut eng, 1, p, BugSource::Ability(ability));
        assert!(status(&eng, p).contains(Status::Bugged));

        eng.execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: 2,
            payload: Action::ArchiveBug(ArchiveBug { bug_id: bug }),
        })
        .unwrap();
        assert!(!status(&eng, p).contains(Status::Bugged));
    }

    // A custody bug is incidental to being held; the `Custody` flag already carries that, so it must
    // not also raise `Bugged` — that would just restate custody as surveillance.
    #[test]
    fn a_custody_bug_does_not_set_bugged() {
        let (mut eng, p) = engine_with_player();

        plant_bug(&mut eng, 1, p, BugSource::Custody);
        assert!(!status(&eng, p).contains(Status::Bugged));
    }

    // A death that happens in the dark surfaces only as `missing`: the world learns someone is gone,
    // not that they died or why.
    #[test]
    fn a_new_death_during_a_blackout_shows_only_missing() {
        let (mut eng, p) = engine_with_player();
        set_blackout(&mut eng, 1, true);
        quick_kill(&mut eng, 2, false, true, false, p);

        let s = status(&eng, p);
        assert!(s.contains(Status::Missing));
        assert!(!s.contains(Status::Dead));
    }

    // A death the world already saw is never retracted by a later blackout.
    #[test]
    fn a_death_known_before_a_blackout_stays_visible() {
        let (mut eng, p) = engine_with_player();
        quick_kill(&mut eng, 1, false, true, false, p);
        assert!(status(&eng, p).contains(Status::Dead));

        set_blackout(&mut eng, 2, true);
        let s = status(&eng, p);
        assert!(s.contains(Status::Dead));
        assert!(!s.contains(Status::Missing));
    }

    #[test]
    fn lifting_a_blackout_reveals_a_hidden_death() {
        let (mut eng, p) = engine_with_player();
        set_blackout(&mut eng, 1, true);
        quick_kill(&mut eng, 2, false, true, false, p);
        assert!(status(&eng, p).contains(Status::Missing) && !status(&eng, p).contains(Status::Dead));

        set_blackout(&mut eng, 3, false);
        let s = status(&eng, p);
        assert!(s.contains(Status::Dead));
        assert!(!s.contains(Status::Missing));
    }
}
