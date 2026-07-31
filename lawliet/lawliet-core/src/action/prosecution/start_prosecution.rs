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
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, SetCustody,
    },
    actor::modifier::Modifier,
    common::{ProsecutionKey, TimerKey, Version},
    engine::Engine,
    helpers::{get_actor, require_player},
    prosecution::{
        Prosecution, ProsecutionDefense, ProsecutionPhase, ProsecutionProsecutor, ProsecutionSide,
    },
};

use lawliet_types::command::{Command, CommandRecipient};

use super::schedule_advance;

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

        if self.prosecutor_id == self.defendant_id {
            return Err(ActionError::CannotProsecuteSelf);
        }

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
            let prosecution_id = eng.world.add_prosecution(Prosecution {
                source: self.source,
                prosecution: ProsecutionProsecutor {
                    prosecutor: self.prosecutor_id,
                    prosecutor_display: self.prosecutor_display,
                },
                defense: ProsecutionDefense {
                    defendant: self.defendant_id,
                    defendant_display: self.defendant_display,
                    lawyer: None,
                },
                phase: ProsecutionPhase::Custody {
                    prosecutor_ready: false,
                    defense_ready: false,
                    timer: TimerKey::default(),
                },
                autonomous: self.autonomous,
                pending_advance: false,
            });

            // The countdown fires at the prosecution, so it cannot exist until the prosecution has
            // a key — hence the placeholder above and the write-back here.
            let timer_id =
                schedule_advance(eng, prosecution_id, eng.config.defaults.custody_timeout);

            if let ProsecutionPhase::Custody { ref mut timer, .. } = eng
                .world
                .get_prosecution_mut(prosecution_id)
                .expect("just inserted")
                .phase
            {
                *timer = timer_id;
            }

            // Tell each side which side they are. Before the snapshot, which the trailing Update
            // step broadcasts — though nothing depends on that order: this names a prosecution
            // rather than describing one, so it stands on its own whichever arrives first.
            for (player_id, side) in [
                (self.prosecutor_id, ProsecutionSide::Prosecutor),
                (self.defendant_id, ProsecutionSide::Defendant),
            ] {
                ctx.push_cmd(
                    Command::InProsecution {
                        prosecution_id,
                        side,
                    },
                    CommandRecipient::Actor(player_id),
                    eng.time,
                );
            }

            prosecution_id
        } else {
            ProsecutionKey::default()
        };

        // The custody announcement is broadcast by UpdateProsecutions in the trailing Update step.

        Ok(ActionResponse::StartProsecution(StartProsecutionResponse {
            id,
        }))
    }
}
