/*
* SYSTEM / ADMIN ACTION
* Put a player into or out of custody.
*
* On: adds State::Custody and creates a BugSource::Custody bug.
*     Fails with AlreadyADefendant if the player is already in custody.
* Off: archives the active custody bug and removes State::Custody.
*     Fails with NotInProsecution if the player is not in custody.
*
* Called by StartProsecution (on) and TerminateProsecution (off).
*/

use smallvec::SmallVec;

use crate::{
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionError, ActionResponse, AddState, RemoveState, ArchiveBug, CreateBug,
    },
    actor::state::State,
    bug::BugSource,
    common::Version,
    engine::Engine,
    helpers::{get_actor, get_player, require_player},
};

pub use crate::action::{SetCustody, SetCustodyResponse};

impl ActionInterface for SetCustody {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        require_player(eng, self.defendant_id)?;
        let in_custody = get_actor(eng, self.defendant_id)
            .expect("already validated")
            .has_state(State::Custody);

        if self.custody {
            if in_custody {
                return Err(ActionError::AlreadyADefendant);
            }

            Action::AddState(AddState {
                actor_id: self.defendant_id,
                state: State::Custody,
            })
            .handle(eng, ctx, actor, version, mutate)?;

            Action::CreateBug(CreateBug {
                target_id: self.defendant_id,
                source: BugSource::Custody,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        } else {
            if !in_custody {
                return Err(ActionError::NotInProsecution);
            }

            let bug_ids: SmallVec<[_; 4]> = get_player(eng, self.defendant_id)
                .expect("already validated as player")
                .bugs
                .iter()
                .copied()
                .collect();

            let bug_id = bug_ids
                .into_iter()
                .find(|&id| {
                    let bug = eng.world.get_bug(id).expect("stale bug id");
                    bug.source == BugSource::Custody && bug.enabled
                })
                .expect("in custody but no active custody bug");

            Action::ArchiveBug(ArchiveBug { bug_id })
                .handle(eng, ctx, actor, version, mutate)?;

            Action::RemoveState(RemoveState {
                actor_id: self.defendant_id,
                state: State::Custody,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::SetCustody(SetCustodyResponse {}))
    }
}
