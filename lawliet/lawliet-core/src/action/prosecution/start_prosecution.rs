/*
* SYSTEM ACTION
* Start a prosecution. Validates preconditions, puts the defendant in custody,
* creates the prosecution object, and schedules the custody timeout.
*
* Displays are passed in here because this action is called from many different prosecution
* abilities (Prosecute, AnonymousProsecute, SilentProsecute, etc.) which each control how
* the prosecutor appears. They are stored in the trial channel's member data, not in the
* Prosecution struct.
*
* Preconditions:
* - prosecutor exists, is a player, and has presence
* - defendant exists, is a player, and has presence
*   (SetCustody handles the already-a-defendant check)
*
* On execution:
* - SetCustody { defendant, custody: true }
* - store Prosecution in world, schedule custody timeout → AdvanceProsecution
* - TODO: commands
*/

use crate::{
    ActorKey,
    action::{
        ActionContext, ActionInterface, ActionResult, Action, ActionActor, ActionError, ActionRequest, ActionResponse, AdvanceProsecution, SetCustody,
    },
    actor::{ActorDisplay, modifier::Modifier},
    common::{ProsecutionKey, Version},
    engine::Engine,
    helpers::{get_actor, require_player},
    prosecution::{Prosecution, ProsecutionDefense, ProsecutionPhase, ProsecutionSource},
};

pub use crate::action::{StartProsecution, StartProsecutionResponse};

impl ActionInterface for StartProsecution {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        require_player(eng, self.prosecutor_id)?;
        if get_actor(eng, self.prosecutor_id)
            .expect("already validated")
            .has_modifier(Modifier::NoPresence)
        {
            return Err(ActionError::UserNotPresent);
        }

        require_player(eng, self.defendant_id)?;
        if get_actor(eng, self.defendant_id)
            .expect("already validated")
            .has_modifier(Modifier::NoPresence)
        {
            return Err(ActionError::UserNotPresent);
        }

        Action::SetCustody(SetCustody {
            defendant_id: self.defendant_id,
            custody: true,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        let id = if mutate {
            let custody_timeout = eng.time + eng.config.defaults.custody_timeout;

            let prosecution_id = eng.world.add_prosecution(Prosecution {
                source: self.source,
                prosecutor: self.prosecutor_id,
                defense: ProsecutionDefense {
                    defendant: self.defendant_id,
                    lawyer: None,
                },
                phase: ProsecutionPhase::Custody {
                    prosecutor_ready: false,
                    defense_ready: false,
                    timeout_job_id: 0,
                },
                autonomous: self.autonomous,
            });

            let job_id = eng.jobs.push(ActionRequest {
                actor: ActionActor::System,
                timestamp: custody_timeout,
                payload: Action::AdvanceProsecution(AdvanceProsecution { prosecution_id }),
            });

            if let ProsecutionPhase::Custody {
                ref mut timeout_job_id,
                ..
            } = eng
                .world
                .get_prosecution_mut(prosecution_id)
                .expect("just inserted")
                .phase
            {
                *timeout_job_id = job_id;
            }

            prosecution_id
        } else {
            ProsecutionKey::default()
        };

        Ok(ActionResponse::StartProsecution(StartProsecutionResponse {
            id,
        }))
    }
}
