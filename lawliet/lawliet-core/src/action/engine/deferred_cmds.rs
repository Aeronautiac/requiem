/*
* SYSTEM ACTION
* Go through every deferred command in order of lowest to highest time and evaluate.
* On success, the command is pushed to the action context.
*/

// deleting entities while a game is active is forbidden.
// if you wish to do something like this (for instance, give a player a new actor), just kill the
// old one and swap out the player's actor id
// for channels, similarly just remove everyone's permissions

use lawliet_types::command::CommandRecipient;

use crate::{
    action::{ActionInterface, ActionResponse},
    command::DeferredCommand,
};

use crate::action::ActionActor;
pub use crate::action::{DeferredCmds, DeferredCmdsResponse};

impl ActionInterface for DeferredCmds {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let def_cmds = vec![];
        let to_execute: Vec<DeferredCommand> = eng
            .deferred_commands
            .clone()
            .into_iter()
            .map(|i| {
                if matches!(i.payload.recipient, CommandRecipient::Actor(_)) {
                    i
                } else {
                    panic!("Cannot defer commands for non active player targets")
                }
            })
            .collect();

        if mutate {
            eng.deferred_commands = def_cmds;
            for cmd in to_execute {
                ctx.commands.push(cmd.payload);
            }
        }

        Ok(ActionResponse::DeferredCmds(DeferredCmdsResponse {}))
    }
}
