/*
* SYSTEM ACTION
* Go through every deferred command in order of lowest to highest time and evaluate.
* On success, the command is pushed to the action context.
*/

// deleting entities while a game is active is forbidden.
// if you wish to do something like this (for instance, give a player a new actor), just kill the
// old one and swap out the player's actor id
// for channels, similarly just remove everyone's permissions

use crate::{
    action::{
        ActionInterface, ActionResponse,
    },
    actor::modifier::Modifiers,
    command::DeferredCommand,
    helpers::get_actor,
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

        let mut def_cmds = eng.deferred_commands.clone();
        let to_execute: Vec<DeferredCommand> = def_cmds
            .extract_if(.., |cmd| {
                let target_data = get_actor(eng, cmd.payload.recipient.expect("deferred commands should only refer to players. commands with no recipient are considered host commands."))
                    .expect("expected valid actor as a deferred command recipient");
                target_data.modifiers() & cmd.blocking_modifiers == Modifiers::EMPTY
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
