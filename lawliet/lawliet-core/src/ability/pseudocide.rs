use smallvec::SmallVec;

use crate::{
    ability::AbilityInterface,
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface,
        actor::player::{kill::Kill, revive::Revive, schedule_revive::ScheduleRevive},
    },
    common::{AbilityKey, ActorKey},
    config::ability::AbilityName,
    helpers::cmd_world_event,
};

pub use lawliet_types::ability::Pseudocide;
use lawliet_types::command::Command;

impl AbilityInterface for Pseudocide {
    fn ability_name(&self) -> crate::config::ability::AbilityName {
        AbilityName::Pseudocide
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut ActionContext,
        _actor: &crate::action::ActionActor,
        _ability: AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        // `orgs` is client-supplied and is a list, not a map, on the wire (serde_json can't key on
        // an ActorKey). Reject a repeated org so a faker can't submit two conflicting views of the
        // same one — the dedup a map gave for free, done by hand.
        let mut seen: SmallVec<[ActorKey; 8]> = SmallVec::new();
        for (org_id, _) in &self.orgs {
            if seen.contains(org_id) {
                return Err(ActionError::NoDuplicateOrgs);
            }
            seen.push(*org_id);
        }

        Action::Kill(Kill {
            allow_link_chaining: false,
            sever_links: false,
            silent: true,
            set_books_dormant: true,
            death_message: None,
            killer_id: None,
            target_id: self.target_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Action::ScheduleRevive(ScheduleRevive {
            timestamp: eng.time + eng.config.defaults.pseudocide_duration,
            revive: Revive {
                ignore_links: true,
                target_id: self.target_id,
            },
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        cmd_world_event(
            eng,
            ctx,
            Command::Death {
                target_id: self.target_id,
                true_name: self.true_name.to_lowercase(),
                death_message: if let Some(msg) = &self.death_message {
                    msg.clone()
                } else {
                    eng.config.defaults.death_message.clone()
                },
                role: self.role,
                notebook_transferred: self.notebook_transferred,
                ability_transferred: self.ability_transferred,
                orgs: self.orgs.clone(),
            },
        );

        Ok(super::AbilityStatus::Success)
    }
}
