/*
* SYSTEM ACTION
* Revive a dead player
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        RemoveState, ReturnDormantBooks,
    },
    actor::{ActorLinkType, state::State},
    common::Version,
    engine::Engine,
    helpers::{cmd_world_event, get_actor, require_dead},
};

pub use crate::action::{Revive, ReviveResponse};
use lawliet_types::command::Command;

impl ActionInterface for Revive {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;
        require_dead(eng, self.target_id)?;

        Action::RemoveState(RemoveState {
            actor_id: self.target_id,
            state: State::Dead,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        Action::ReturnDormantBooks(ReturnDormantBooks {
            actor_id: self.target_id,
        })
        .handle(eng, ctx, actor, version, mutate)?;

        // The revived player is back in the presence viewport (RemoveState cleared their death),
        // so they hear the announcement with everyone else. Mirrors Death: a generic announcement
        // carrying a message the caller may set, defaulting to the configured revival message, and
        // skipped when the revive is silent.
        if !self.silent {
            let message = if let Some(msg) = &self.revival_message {
                msg.clone()
            } else {
                eng.config.defaults.revival_message.clone()
            };
            cmd_world_event(
                eng,
                ctx,
                Command::Revival {
                    target_id: self.target_id,
                    message,
                },
            );
        }

        let mut next_actions = vec![];
        if !self.ignore_links {
            let actor = get_actor(eng, self.target_id)?;
            let links = actor.actor_links.clone();
            for link in links {
                if link.link_type == ActorLinkType::Life {
                    let other_actor = get_actor(eng, link.link_dest)?;
                    if other_actor.states.contains(State::Dead) {
                        next_actions.push(Action::Revive(Revive {
                            ignore_links: false,
                            silent: self.silent,
                            revival_message: self.revival_message.clone(),
                            target_id: link.link_dest,
                        }));
                    }
                }
            }
        }
        for mut action in next_actions {
            action.handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::Revive(ReviveResponse {}))
    }
}
