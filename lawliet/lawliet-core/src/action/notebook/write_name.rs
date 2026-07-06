/*
* PLAYER ACTION
* Write a player's name in a notebook
* IPP blocks this
*/

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, Kill, ScheduleKill,
    },
    actor::modifier::Modifier,
    command::Command,
    common::Version,
    engine::Engine,
    helpers::{actor_get_effective_passive, actor_id, get_actor, get_notebook, get_notebook_mut},
    notebook::NotebookError,
    passive::PassiveType,
};

pub use crate::action::{WriteName, WriteNameResponse};

impl ActionInterface for WriteName {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.player_only()?;
        let player_id = actor_id(actor).unwrap();
        let target = eng.world.get_player_id_by_name(&self.true_name);

        let player_actor = eng.world.get_actor_mut(player_id).unwrap();
        if player_actor.has_modifier(Modifier::NoNotebookUsage) {
            return Err(ActionError::NotebookUsageBlocked);
        }

        let book = get_notebook(eng, self.notebook_id)?;
        if actor_get_effective_passive(eng, player_id, |passive| {
            matches!(passive, PassiveType::OwnedNotebookBlock)
        })
        .is_some()
            && book.original_owner == Some(player_id)
        {
            return Err(ActionError::NotebookUsageBlocked);
        }

        let success_limit = eng.config.defaults.notebook_successes_per_day;
        let failure_limit = eng.config.defaults.notebook_failures_per_day;
        if let Err(error) = book.can_write(player_id, failure_limit, success_limit) {
            return Err(match error {
                NotebookError::NoOwner | NotebookError::NotOwned => ActionError::NotebookNotOwned,
                NotebookError::OnCooldown => ActionError::NotebookOnCooldown,
            });
        }

        if let Some(target_id) = target
            && !book.fake
        {
            let cancelled = eng.jobs.cancel_all_cond(
                |job| {
                    if let Action::NotebookScheduledKill(data) = &job.request.payload {
                        data.kill.target_id == target_id
                    } else {
                        false
                    }
                },
                mutate,
            ) > 0;

            let book = get_notebook_mut(eng, self.notebook_id)?;
            if mutate {
                book.on_write_success(player_id);
            }
            let successes_remaining = book.successes_remaining(player_id, success_limit);
            let attempts_remaining = book.failures_remaining(player_id, failure_limit);

            let kill = Kill {
                allow_link_chaining: true,
                sever_links: true,
                set_books_dormant: false,
                target_id,
                killer_id: Some(player_id),
                death_message: self.death_message.clone(),
                silent: false,
            };

            let mut target_action: Option<Action> = if self.delay > 0 {
                Some(Action::ScheduleKill(ScheduleKill {
                    timestamp: eng.time + self.delay,
                    notebook_scheduled: true,
                    kill,
                }))
            } else {
                Some(Action::Kill(kill))
            };

            if cancelled {
                target_action = None;
            }

            let target_actor =
                get_actor(eng, target_id).expect("expected valid actor in true name map");
            if target_actor.modifiers().contains(Modifier::WriteImmunity) {
                target_action = None;
            }

            if let Some(action) = &mut target_action {
                action.handle(eng, ctx, &ActionActor::System, version, mutate)?;
            }

            ctx.push_cmd(
                Command::NotebookWrite {
                    notebook_id: self.notebook_id,
                    delay: self.delay,
                    message: self.death_message.clone(),
                    true_name: self.true_name.clone(),
                    success: true,
                    target_saved: target_action.is_none(),
                    successes_remaining,
                    attempts_remaining,
                    user_id: player_id,
                },
                CommandRecipient::System,
                eng.time,
            );
        } else {
            let book = get_notebook_mut(eng, self.notebook_id)?;
            if mutate {
                book.on_write_failure(player_id);
            }
            let successes_remaining = book.successes_remaining(player_id, success_limit);
            let attempts_remaining = book.failures_remaining(player_id, failure_limit);

            ctx.push_cmd(
                Command::NotebookWrite {
                    notebook_id: self.notebook_id,
                    delay: self.delay,
                    message: self.death_message.clone(),
                    true_name: self.true_name.clone(),
                    success: false,
                    target_saved: false, // not relevant here
                    successes_remaining,
                    attempts_remaining,
                    user_id: player_id,
                },
                CommandRecipient::System,
                eng.time,
            );
        }

        Ok(ActionResponse::WriteName(WriteNameResponse {}))
    }
}
