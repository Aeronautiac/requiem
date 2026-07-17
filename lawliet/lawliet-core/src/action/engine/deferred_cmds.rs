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

        // A deferred command is delivered only once its recipient has NONE of its blocking
        // modifiers (see DeferredCommand). The canonical case is presence: a broadcast queued
        // with NoPresence must wait while the target lacks presence — delivering it anyway is
        // how e.g. an anonymous announcement leaks to a player who shouldn't yet see it.
        // Anything still blocked stays queued and is re-evaluated next cycle.
        if mutate {
            let mut retained: Vec<DeferredCommand> = vec![];
            for def in eng.deferred_commands.clone() {
                let CommandRecipient::Actor(id) = def.payload.recipient else {
                    panic!("Cannot defer commands for non active player targets");
                };
                // Blocked while the recipient still holds any blocking modifier. If the actor
                // no longer exists it can't be routed, so treat it as unblocked and drop it by
                // delivering (the frontend ignores commands for unknown actors).
                let blocked = get_actor(eng, id)
                    .map(|a| a.modifiers().intersects(def.blocking_modifiers))
                    .unwrap_or(false);
                if blocked {
                    retained.push(def);
                } else {
                    ctx.commands.push(def.payload);
                }
            }
            eng.deferred_commands = retained;
        }

        Ok(ActionResponse::DeferredCmds(DeferredCmdsResponse {}))
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::command::{Command, CommandPayload, CommandRecipient};
    use lawliet_types::role::Role;

    use crate::{
        action::{
            Action, ActionActor, ActionRequest, actor::remove_state::RemoveState,
            engine::null::Null,
        },
        actor::{modifier::Modifier, state::State},
        command::DeferredCommand,
        common::ActorKey,
        engine::Engine,
        test_helpers::{add_player, add_state},
    };

    // Queue an anonymous-announcement command deferred behind presence for `target`.
    fn queue_presence_gated(eng: &mut Engine, target: ActorKey) {
        eng.deferred_commands.push(DeferredCommand {
            payload: CommandPayload {
                timestamp: eng.time,
                recipient: CommandRecipient::Actor(target),
                cmd: Command::AnonymousAnnouncement {
                    content: "psst".into(),
                },
            },
            blocking_modifiers: Modifier::NoPresence.into(),
        });
    }

    fn delivered_to(commands: &[CommandPayload], who: ActorKey) -> bool {
        commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(who)
                && matches!(&p.cmd, Command::AnonymousAnnouncement { .. })
        })
    }

    #[test]
    fn deferred_command_withheld_until_blocking_modifier_clears() {
        let mut eng = Engine::new();
        let present = add_player(&mut eng, 0, Role::Civilian, "present");
        let absent = add_player(&mut eng, 0, Role::Civilian, "absent");
        // Incarceration grants NoPresence: `absent` should not receive presence-gated
        // broadcasts until released.
        add_state(&mut eng, 0, absent, State::Incarcerated);

        queue_presence_gated(&mut eng, present);
        queue_presence_gated(&mut eng, absent);

        // Any action triggers Update -> DeferredCmds; a Null flush is enough.
        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::Null(Null {}),
            })
            .unwrap();

        // Present player gets it; the no-presence player does not, and their copy stays queued.
        assert!(delivered_to(&ctx.commands, present));
        assert!(!delivered_to(&ctx.commands, absent));
        assert!(
            eng.deferred_commands
                .iter()
                .any(|d| d.payload.recipient == CommandRecipient::Actor(absent))
        );
        assert!(
            !eng.deferred_commands
                .iter()
                .any(|d| d.payload.recipient == CommandRecipient::Actor(present))
        );

        // Releasing them (clearing NoPresence) lets the next action's flush deliver it.
        let (_, ctx) = eng
            .execute(ActionRequest {
                actor: ActionActor::System,
                timestamp: 0,
                payload: Action::RemoveState(RemoveState {
                    actor_id: absent,
                    state: State::Incarcerated,
                }),
            })
            .unwrap();

        assert!(delivered_to(&ctx.commands, absent));
        assert!(eng.deferred_commands.is_empty());
    }
}
